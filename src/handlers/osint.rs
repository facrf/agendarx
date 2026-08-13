use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use reqwest::{Client, Url, redirect::Policy};
use tokio::net::lookup_host;

mod providers;

use providers::{PublicSearchProviders, SearchProvider};

use crate::{
    AppState,
    error::AppError,
    models::{
        HistoricoBuscaPublica, HistoricoBuscaResponse, ParametroBusca, ParametroBuscaInput,
        VarreduraResponse,
    },
};

const TIPOS_VALIDOS: [&str; 6] = ["NOME", "CPF", "CNPJ", "EMAIL", "TELEFONE", "TERMO"];
const MAX_PARAMETROS_ATIVOS: usize = 50;
const USER_AGENT: &str = "AgendarX-OSINT/0.3 (+arquivamento-publico)";

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route(
            "/parametros/{pessoa_id}",
            get(listar_parametros).post(criar_parametro),
        )
        .route(
            "/parametros/item/{id}",
            axum::routing::put(atualizar_parametro).delete(excluir_parametro),
        )
        .route("/varrer/{pessoa_id}", post(varrer))
        .route("/historico/{pessoa_id}", get(listar_historico))
}

async fn listar_parametros(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<ParametroBusca>>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let parametros = sqlx::query_as::<_, ParametroBusca>(
        "SELECT id, pessoa_id, tipo, valor, provider, ativo FROM parametro_busca \
         WHERE pessoa_id = ? ORDER BY ativo DESC, provider, tipo, valor",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(parametros))
}

async fn criar_parametro(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    Json(input): Json<ParametroBuscaInput>,
) -> Result<(StatusCode, Json<ParametroBusca>), AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let (tipo, valor, provider, ativo) = normalizar_parametro(input)?;
    let parametro = sqlx::query_as::<_, ParametroBusca>(
        "INSERT INTO parametro_busca (pessoa_id, tipo, valor, provider, ativo) VALUES (?, ?, ?, ?, ?) \
         RETURNING id, pessoa_id, tipo, valor, provider, ativo",
    )
    .bind(pessoa_id)
    .bind(tipo)
    .bind(valor)
    .bind(provider)
    .bind(ativo)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(parametro)))
}

async fn atualizar_parametro(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<ParametroBuscaInput>,
) -> Result<Json<ParametroBusca>, AppError> {
    let (tipo, valor, provider, ativo) = normalizar_parametro(input)?;
    let parametro = sqlx::query_as::<_, ParametroBusca>(
        "UPDATE parametro_busca SET tipo = ?, valor = ?, provider = ?, ativo = ? WHERE id = ? \
         RETURNING id, pessoa_id, tipo, valor, provider, ativo",
    )
    .bind(tipo)
    .bind(valor)
    .bind(provider)
    .bind(ativo)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("parâmetro de busca"))?;
    Ok(Json(parametro))
}

async fn excluir_parametro(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM parametro_busca WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("parâmetro de busca"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn listar_historico(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<HistoricoBuscaResponse>>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let itens = sqlx::query_as::<_, HistoricoBuscaPublica>(
        "SELECT id, pessoa_id, fonte, provider, parametro_utilizado, titulo_resultado, snippet, \
                url_origem, anexo_dossie_id, data_publicacao, detalhes, data_captura \
         FROM historico_busca_publica WHERE pessoa_id = ? \
         ORDER BY data_captura DESC, id DESC",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        itens
            .into_iter()
            .map(|item| HistoricoBuscaResponse {
                id: item.id,
                pessoa_id: item.pessoa_id,
                fonte: item.fonte,
                provider: item.provider,
                parametro_utilizado: item.parametro_utilizado,
                titulo_resultado: item.titulo_resultado,
                snippet: item.snippet,
                url_origem: item.url_origem,
                url_pdf: item
                    .anexo_dossie_id
                    .map(|id| format!("/api/dossie/anexos/{id}/stream")),
                anexo_dossie_id: item.anexo_dossie_id,
                data_publicacao: item.data_publicacao,
                detalhes: item.detalhes,
                data_captura: item.data_captura,
            })
            .collect(),
    ))
}

async fn varrer(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<VarreduraResponse>, AppError> {
    let inicio_varredura = Instant::now();
    garantir_pessoa(&state, pessoa_id).await?;
    let parametros = sqlx::query_as::<_, ParametroBusca>(
        "SELECT id, pessoa_id, tipo, valor, provider, ativo FROM parametro_busca \
         WHERE pessoa_id = ? AND ativo = 1 ORDER BY id",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;

    if parametros.is_empty() {
        return Err(AppError::BadRequest(
            "adicione e ative ao menos um parâmetro antes da varredura".to_owned(),
        ));
    }
    if parametros.len() > MAX_PARAMETROS_ATIVOS {
        return Err(AppError::BadRequest(format!(
            "a varredura aceita no máximo {MAX_PARAMETROS_ATIVOS} parâmetros ativos"
        )));
    }

    let mut providers = PublicSearchProviders::new(&state.config).map_err(AppError::interno)?;

    let mut resposta = VarreduraResponse {
        situacao: "concluida".to_owned(),
        parametros_processados: parametros.len(),
        parametros_inconclusivos: 0,
        resultados_encontrados: 0,
        novos_achados: 0,
        pdfs_arquivados: 0,
        fontes_indisponiveis: 0,
        avisos: Vec::new(),
    };
    let mut fontes_indisponiveis = HashSet::new();
    let mut varredura_degradada = false;

    for parametro in parametros {
        let inicio_parametro = Instant::now();
        let provider = match SearchProvider::parse(&parametro.provider) {
            Ok(provider) => provider,
            Err(error) => {
                resposta.parametros_inconclusivos += 1;
                varredura_degradada = true;
                resposta.avisos.push(format!(
                    "pesquisa {} ignorada por possuir uma fonte inválida: {error}",
                    parametro.id
                ));
                tracing::warn!(
                    pessoa_id,
                    parametro_id = parametro.id,
                    provider = %parametro.provider,
                    "provider inválido em parâmetro persistido"
                );
                continue;
            }
        };
        let execution = providers.search(provider, &parametro).await;
        for source in &execution.unavailable_sources {
            fontes_indisponiveis.insert(source.clone());
        }
        resposta.avisos.extend(execution.warnings);
        if execution.inconclusive {
            resposta.parametros_inconclusivos += 1;
        }
        varredura_degradada |= execution.degraded;
        resposta.resultados_encontrados += execution.results.len();

        if execution.inconclusive {
            tracing::warn!(
                pessoa_id,
                parametro_id = parametro.id,
                tipo = %parametro.tipo,
                provider = provider.as_str(),
                consultas_executadas = execution.requests_executed,
                resultados = execution.results.len(),
                fontes_indisponiveis = execution.unavailable_sources.len(),
                duracao_ms = inicio_parametro.elapsed().as_millis(),
                "parâmetro OSINT processado de forma inconclusiva"
            );
        } else {
            tracing::info!(
                pessoa_id,
                parametro_id = parametro.id,
                tipo = %parametro.tipo,
                provider = provider.as_str(),
                consultas_executadas = execution.requests_executed,
                resultados = execution.results.len(),
                fontes_indisponiveis = execution.unavailable_sources.len(),
                duracao_ms = inicio_parametro.elapsed().as_millis(),
                "parâmetro OSINT processado"
            );
        }

        for result in execution.results {
            let url_texto = result.url.trim();
            if url_texto.is_empty() || url_texto.len() > 4096 {
                continue;
            }
            let ja_existe: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM historico_busca_publica \
                 WHERE pessoa_id = ? AND url_origem = ?)",
            )
            .bind(pessoa_id)
            .bind(url_texto)
            .fetch_one(&state.pool)
            .await?;
            if ja_existe {
                continue;
            }

            let url = match Url::parse(url_texto) {
                Ok(url) if matches!(url.scheme(), "http" | "https") => url,
                _ => {
                    resposta
                        .avisos
                        .push("um resultado com URL inválida foi ignorado".to_owned());
                    continue;
                }
            };
            let mut pdf = None;
            if url.path().to_ascii_lowercase().ends_with(".pdf") {
                match baixar_pdf(&state, &url).await {
                    Ok(arquivo) => pdf = Some(arquivo),
                    Err(erro) => resposta.avisos.push(format!(
                        "PDF não arquivado em {}: {erro}",
                        resumir_url(&url)
                    )),
                }
            }

            let parametro_utilizado = format!("{}: {}", parametro.tipo, parametro.valor);
            let (novo, pdf_salvo) = registrar_achado(
                &state,
                NovoAchado {
                    pessoa_id,
                    provider: provider.as_str(),
                    fonte: &result.source,
                    parametro_utilizado: &parametro_utilizado,
                    titulo: &result.title,
                    snippet: result.description.as_deref(),
                    url_origem: url.as_str(),
                    data_publicacao: result.published_at.as_deref(),
                    detalhes: result.details.as_deref(),
                    pdf,
                },
            )
            .await?;
            resposta.novos_achados += usize::from(novo);
            resposta.pdfs_arquivados += usize::from(pdf_salvo);
        }
    }

    resposta.fontes_indisponiveis = fontes_indisponiveis.len();
    resposta.situacao = classificar_varredura(
        resposta.parametros_processados,
        resposta.parametros_inconclusivos,
        varredura_degradada,
    )
    .to_owned();
    tracing::info!(
        pessoa_id,
        situacao = %resposta.situacao,
        parametros_processados = resposta.parametros_processados,
        parametros_inconclusivos = resposta.parametros_inconclusivos,
        resultados_encontrados = resposta.resultados_encontrados,
        novos_achados = resposta.novos_achados,
        pdfs_arquivados = resposta.pdfs_arquivados,
        fontes_indisponiveis = resposta.fontes_indisponiveis,
        avisos = resposta.avisos.len(),
        duracao_ms = inicio_varredura.elapsed().as_millis(),
        "varredura OSINT concluída"
    );

    Ok(Json(resposta))
}

struct NovoAchado<'a> {
    pessoa_id: i64,
    provider: &'a str,
    fonte: &'a str,
    parametro_utilizado: &'a str,
    titulo: &'a str,
    snippet: Option<&'a str>,
    url_origem: &'a str,
    data_publicacao: Option<&'a str>,
    detalhes: Option<&'a str>,
    pdf: Option<PdfBaixado>,
}

async fn registrar_achado(
    state: &AppState,
    achado: NovoAchado<'_>,
) -> Result<(bool, bool), AppError> {
    let NovoAchado {
        pessoa_id,
        provider,
        fonte,
        parametro_utilizado,
        titulo,
        snippet,
        url_origem,
        data_publicacao,
        detalhes,
        pdf,
    } = achado;
    let mut transacao = state.pool.begin().await?;
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM historico_busca_publica \
         WHERE pessoa_id = ? AND url_origem = ?)",
    )
    .bind(pessoa_id)
    .bind(url_origem)
    .fetch_one(&mut *transacao)
    .await?;
    if existe {
        transacao.rollback().await?;
        return Ok((false, false));
    }

    let tinha_pdf = pdf.is_some();
    let anexo_id = if let Some(pdf) = pdf {
        let tamanho = pdf.conteudo.len() as i64;
        Some(
            sqlx::query_scalar::<_, i64>(
                "INSERT INTO anexo_dossie \
                    (pessoa_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes) \
                 VALUES (?, ?, 'application/pdf', ?, ?) RETURNING id",
            )
            .bind(pessoa_id)
            .bind(pdf.nome_arquivo)
            .bind(pdf.conteudo)
            .bind(tamanho)
            .fetch_one(&mut *transacao)
            .await?,
        )
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO historico_busca_publica \
            (pessoa_id, provider, fonte, parametro_utilizado, titulo_resultado, snippet, \
             url_origem, anexo_dossie_id, data_publicacao, detalhes) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(pessoa_id)
    .bind(provider)
    .bind(truncar(fonte, 200))
    .bind(truncar(parametro_utilizado, 600))
    .bind(truncar(titulo, 1000))
    .bind(snippet.map(|valor| truncar(valor, 4000)))
    .bind(url_origem)
    .bind(anexo_id)
    .bind(data_publicacao.map(|valor| truncar(valor, 40)))
    .bind(detalhes.map(|valor| truncar(valor, 4000)))
    .execute(&mut *transacao)
    .await?;
    transacao.commit().await?;
    Ok((true, tinha_pdf))
}

async fn baixar_pdf(state: &AppState, url: &Url) -> Result<PdfBaixado, String> {
    let (host, enderecos) = resolver_destino_publico(url).await?;
    let mut construtor = Client::builder()
        .timeout(Duration::from_secs(state.config.osint_timeout_seconds))
        .redirect(Policy::none())
        .no_proxy()
        .user_agent(USER_AGENT);
    if !enderecos.is_empty() {
        construtor = construtor.resolve_to_addrs(&host, &enderecos);
    }
    let cliente = construtor
        .build()
        .map_err(|erro| format!("falha ao configurar download: {erro}"))?;
    let mut response = cliente
        .get(url.clone())
        .header(reqwest::header::ACCEPT, "application/pdf")
        .send()
        .await
        .map_err(|erro| format!("download falhou: {erro}"))?;
    if !response.status().is_success() {
        return Err(format!("servidor respondeu com HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|tamanho| tamanho > state.config.osint_max_pdf_bytes as u64)
    {
        return Err("arquivo excede o limite configurado".to_owned());
    }

    let mut conteudo = Vec::new();
    while let Some(parte) = response
        .chunk()
        .await
        .map_err(|erro| format!("falha ao receber o arquivo: {erro}"))?
    {
        if conteudo.len().saturating_add(parte.len()) > state.config.osint_max_pdf_bytes {
            return Err("arquivo excede o limite configurado".to_owned());
        }
        conteudo.extend_from_slice(&parte);
    }
    let possui_assinatura_pdf = conteudo
        .windows(5)
        .take(1024)
        .any(|janela| janela == b"%PDF-");
    if conteudo.is_empty() || !possui_assinatura_pdf {
        return Err("o conteúdo recebido não é um PDF válido".to_owned());
    }

    Ok(PdfBaixado {
        nome_arquivo: nome_pdf(url),
        conteudo,
    })
}

async fn resolver_destino_publico(url: &Url) -> Result<(String, Vec<SocketAddr>), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err("somente URLs HTTP ou HTTPS são aceitas".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL com credenciais embutidas não é aceita".to_owned());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "URL sem endereço de destino".to_owned())?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return Err("destinos locais não podem ser arquivados".to_owned());
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if !ip_publico(ip) {
            return Err("endereços IP locais, privados ou reservados são bloqueados".to_owned());
        }
        return Ok((host, Vec::new()));
    }

    let porta = url
        .port_or_known_default()
        .ok_or_else(|| "porta de destino inválida".to_owned())?;
    let enderecos: Vec<SocketAddr> = lookup_host((host.as_str(), porta))
        .await
        .map_err(|erro| format!("não foi possível resolver o endereço: {erro}"))?
        .collect();
    if enderecos.is_empty() {
        return Err("o endereço não possui IP resolvível".to_owned());
    }
    if enderecos.iter().any(|endereco| !ip_publico(endereco.ip())) {
        return Err("o domínio resolve para um endereço local, privado ou reservado".to_owned());
    }
    Ok((host, enderecos))
}

fn ip_publico(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ipv4_publico(ip),
        IpAddr::V6(ip) => ipv6_publico(ip),
    }
}

fn ipv4_publico(ip: Ipv4Addr) -> bool {
    let octetos = ip.octets();
    octetos[0] != 0
        && octetos[0] < 240
        && !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_broadcast()
        && !ip.is_documentation()
        && !ip.is_unspecified()
        && !ip.is_multicast()
        && !(octetos[0] == 100 && (64..=127).contains(&octetos[1]))
        && !(octetos[0] == 192 && octetos[1] == 0 && octetos[2] == 0)
        && !(octetos[0] == 198 && (octetos[1] == 18 || octetos[1] == 19))
}

fn ipv6_publico(ip: Ipv6Addr) -> bool {
    if let Some(ipv4) = ip.to_ipv4() {
        return ipv4_publico(ipv4);
    }
    let segmentos = ip.segments();
    !ip.is_loopback()
        && !ip.is_unspecified()
        && !ip.is_multicast()
        && segmentos[0] & 0xfe00 != 0xfc00
        && segmentos[0] & 0xffc0 != 0xfe80
        && segmentos[0] & 0xffc0 != 0xfec0
        && !(segmentos[0] == 0x2001 && segmentos[1] == 0x0db8)
}

fn normalizar_parametro(
    input: ParametroBuscaInput,
) -> Result<(String, String, String, bool), AppError> {
    let tipo = input.tipo.trim().to_ascii_uppercase();
    if !TIPOS_VALIDOS.contains(&tipo.as_str()) {
        return Err(AppError::BadRequest(format!(
            "tipo inválido; use {}",
            TIPOS_VALIDOS.join(", ")
        )));
    }
    let valor = input.valor.trim().to_owned();
    if valor.is_empty() || valor.chars().count() > 512 {
        return Err(AppError::BadRequest(
            "o valor deve possuir entre 1 e 512 caracteres".to_owned(),
        ));
    }
    let quantidade_digitos = valor.chars().filter(char::is_ascii_digit).count();
    if tipo == "CPF" && quantidade_digitos != 11 {
        return Err(AppError::BadRequest(
            "CPF deve possuir 11 dígitos".to_owned(),
        ));
    }
    if tipo == "CNPJ" && quantidade_digitos != 14 {
        return Err(AppError::BadRequest(
            "CNPJ deve possuir 14 dígitos".to_owned(),
        ));
    }
    if tipo == "EMAIL" && (!valor.contains('@') || valor.contains(char::is_whitespace)) {
        return Err(AppError::BadRequest("e-mail inválido".to_owned()));
    }
    if tipo == "TELEFONE" && !(7..=15).contains(&quantidade_digitos) {
        return Err(AppError::BadRequest(
            "telefone deve possuir entre 7 e 15 dígitos".to_owned(),
        ));
    }
    let provider = SearchProvider::parse(&input.provider).map_err(AppError::BadRequest)?;
    Ok((
        tipo,
        valor,
        provider.as_str().to_owned(),
        input.ativo.unwrap_or(true),
    ))
}

fn classificar_varredura(
    parametros_processados: usize,
    parametros_inconclusivos: usize,
    degradada: bool,
) -> &'static str {
    if parametros_processados > 0 && parametros_inconclusivos == parametros_processados {
        "inconclusiva"
    } else if degradada {
        "parcial"
    } else {
        "concluida"
    }
}

fn nome_pdf(url: &Url) -> String {
    let original = url
        .path_segments()
        .and_then(|mut segmentos| segmentos.next_back())
        .filter(|nome| !nome.is_empty())
        .unwrap_or("achado-osint.pdf");
    let mut seguro: String = original
        .chars()
        .map(|caractere| {
            if caractere.is_ascii_alphanumeric() || ".-_".contains(caractere) {
                caractere
            } else {
                '_'
            }
        })
        .take(220)
        .collect();
    if !seguro.to_ascii_lowercase().ends_with(".pdf") {
        seguro.push_str(".pdf");
    }
    seguro
}

fn resumir_url(url: &Url) -> String {
    format!(
        "{}{}",
        url.host_str().unwrap_or("fonte externa"),
        truncar(url.path(), 80)
    )
}

fn truncar(valor: &str, maximo: usize) -> String {
    valor.chars().take(maximo).collect()
}

struct PdfBaixado {
    nome_arquivo: String,
    conteudo: Vec<u8>,
}

async fn garantir_pessoa(state: &AppState, id: i64) -> Result<(), AppError> {
    let existe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ?)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if !existe {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{classificar_varredura, ipv4_publico, ipv6_publico, normalizar_parametro};
    use crate::models::ParametroBuscaInput;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn configuracao_antiga_usa_searxng() {
        let input: ParametroBuscaInput =
            serde_json::from_str(r#"{"tipo":"TERMO","valor":"licitação","ativo":true}"#).unwrap();
        let (_, _, provider, _) = normalizar_parametro(input).unwrap();
        assert_eq!(provider, "SEARXNG");
    }

    #[test]
    fn normalizacao_preserva_cada_provider() {
        for provider in ["SEARXNG", "QUERIDO_DIARIO", "INLABS", "OPENALEX"] {
            let input: ParametroBuscaInput = serde_json::from_value(serde_json::json!({
                "tipo": "TERMO",
                "valor": "pesquisa",
                "provider": provider
            }))
            .unwrap();
            let (_, _, normalized, _) = normalizar_parametro(input).unwrap();
            assert_eq!(normalized, provider);
        }
    }

    #[test]
    fn provider_invalido_retorna_erro_sem_panic() {
        let input: ParametroBuscaInput =
            serde_json::from_str(r#"{"tipo":"TERMO","valor":"teste","provider":"URL_ARBITRARIA"}"#)
                .unwrap();
        assert!(normalizar_parametro(input).is_err());
    }

    #[tokio::test]
    async fn migration_persists_defaults_all_providers_and_edits() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let person_id: i64 =
            sqlx::query_scalar("INSERT INTO pessoa (nome) VALUES ('Teste') RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();

        sqlx::query(
            "INSERT INTO parametro_busca (pessoa_id, tipo, valor) VALUES (?, 'TERMO', 'legado')",
        )
        .bind(person_id)
        .execute(&pool)
        .await
        .unwrap();
        let default_provider: String =
            sqlx::query_scalar("SELECT provider FROM parametro_busca WHERE valor = 'legado'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(default_provider, "SEARXNG");

        for provider in ["SEARXNG", "QUERIDO_DIARIO", "INLABS", "OPENALEX"] {
            sqlx::query(
                "INSERT INTO parametro_busca (pessoa_id, tipo, valor, provider) VALUES (?, 'TERMO', 'mesma consulta', ?)",
            )
            .bind(person_id)
            .bind(provider)
            .execute(&pool)
            .await
            .unwrap();
        }
        let saved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM parametro_busca WHERE valor = 'mesma consulta'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(saved, 4);

        sqlx::query(
            "UPDATE parametro_busca SET valor = 'consulta editada' WHERE provider = 'OPENALEX'",
        )
        .execute(&pool)
        .await
        .unwrap();
        let edited_provider: String = sqlx::query_scalar(
            "SELECT provider FROM parametro_busca WHERE valor = 'consulta editada'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(edited_provider, "OPENALEX");

        let invalid = sqlx::query(
            "INSERT INTO parametro_busca (pessoa_id, tipo, valor, provider) VALUES (?, 'TERMO', 'inválida', 'URL')",
        )
        .bind(person_id)
        .execute(&pool)
        .await;
        assert!(invalid.is_err());
    }

    #[test]
    fn classifica_resultado_da_varredura() {
        assert_eq!(classificar_varredura(2, 0, false), "concluida");
        assert_eq!(classificar_varredura(2, 0, true), "parcial");
        assert_eq!(classificar_varredura(2, 1, true), "parcial");
        assert_eq!(classificar_varredura(2, 2, true), "inconclusiva");
    }

    #[test]
    fn bloqueia_ips_nao_publicos() {
        assert!(!ipv4_publico(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!ipv4_publico(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(!ipv4_publico(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(!ipv4_publico(Ipv4Addr::new(0, 1, 2, 3)));
        assert!(!ipv4_publico(Ipv4Addr::new(240, 0, 0, 1)));
        assert!(!ipv6_publico(Ipv6Addr::from_bits(
            0x0000_0000_0000_0000_0000_ffff_7f00_0001,
        )));
        assert!(!ipv6_publico(Ipv6Addr::from_bits(
            0x0000_0000_0000_0000_0000_0000_c0a8_0001,
        )));
        assert!(ipv4_publico(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(ipv6_publico("2606:4700:4700::1111".parse().unwrap()));
    }
}
