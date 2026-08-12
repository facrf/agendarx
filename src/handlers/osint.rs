use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use reqwest::{Client, Url, redirect::Policy};
use serde::Deserialize;
use tokio::net::lookup_host;

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
const USER_AGENT: &str = "AgendarX-OSINT/0.1 (+arquivamento-publico)";

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
        "SELECT id, pessoa_id, tipo, valor, ativo FROM parametro_busca \
         WHERE pessoa_id = ? ORDER BY ativo DESC, tipo, valor",
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
    let (tipo, valor, ativo) = normalizar_parametro(input)?;
    let parametro = sqlx::query_as::<_, ParametroBusca>(
        "INSERT INTO parametro_busca (pessoa_id, tipo, valor, ativo) VALUES (?, ?, ?, ?) \
         RETURNING id, pessoa_id, tipo, valor, ativo",
    )
    .bind(pessoa_id)
    .bind(tipo)
    .bind(valor)
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
    let (tipo, valor, ativo) = normalizar_parametro(input)?;
    let parametro = sqlx::query_as::<_, ParametroBusca>(
        "UPDATE parametro_busca SET tipo = ?, valor = ?, ativo = ? WHERE id = ? \
         RETURNING id, pessoa_id, tipo, valor, ativo",
    )
    .bind(tipo)
    .bind(valor)
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
        "SELECT id, pessoa_id, fonte, parametro_utilizado, titulo_resultado, snippet, \
                url_origem, anexo_dossie_id, data_captura \
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
                parametro_utilizado: item.parametro_utilizado,
                titulo_resultado: item.titulo_resultado,
                snippet: item.snippet,
                url_origem: item.url_origem,
                url_pdf: item
                    .anexo_dossie_id
                    .map(|id| format!("/api/dossie/anexos/{id}/stream")),
                anexo_dossie_id: item.anexo_dossie_id,
                data_captura: item.data_captura,
            })
            .collect(),
    ))
}

async fn varrer(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<VarreduraResponse>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let parametros = sqlx::query_as::<_, ParametroBusca>(
        "SELECT id, pessoa_id, tipo, valor, ativo FROM parametro_busca \
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

    let searxng_url = state.config.searxng_url.as_deref().ok_or_else(|| {
        AppError::ServiceUnavailable(
            "varredura indisponível: configure SEARXNG_URL no servidor".to_owned(),
        )
    })?;
    let endpoint = endpoint_searxng(searxng_url)?;
    let cliente_busca = Client::builder()
        .timeout(Duration::from_secs(state.config.osint_timeout_seconds))
        .redirect(Policy::limited(3))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|erro| AppError::interno(format!("falha ao configurar cliente HTTP: {erro}")))?;

    let mut resposta = VarreduraResponse {
        parametros_processados: parametros.len(),
        resultados_encontrados: 0,
        novos_achados: 0,
        pdfs_arquivados: 0,
        avisos: Vec::new(),
    };

    for parametro in parametros {
        let consulta = consulta_para(&parametro);
        let resultado_busca = cliente_busca
            .get(endpoint.clone())
            .query(&[
                ("q", consulta.as_str()),
                ("format", "json"),
                ("language", "pt-BR"),
                ("safesearch", "0"),
            ])
            .send()
            .await;

        let response = match resultado_busca {
            Ok(response) if response.status().is_success() => response,
            Ok(response) => {
                resposta.avisos.push(format!(
                    "{}: o SearXNG respondeu com HTTP {}",
                    parametro.tipo,
                    response.status()
                ));
                continue;
            }
            Err(erro) => {
                resposta.avisos.push(format!(
                    "{}: não foi possível consultar o SearXNG ({erro})",
                    parametro.tipo
                ));
                continue;
            }
        };

        let dados: SearxResponse = match response.json().await {
            Ok(dados) => dados,
            Err(erro) => {
                resposta.avisos.push(format!(
                    "{}: resposta JSON inválida do SearXNG ({erro})",
                    parametro.tipo
                ));
                continue;
            }
        };
        let resultados: Vec<_> = dados
            .results
            .into_iter()
            .take(state.config.osint_max_results)
            .collect();
        resposta.resultados_encontrados += resultados.len();

        for resultado in resultados {
            let url_texto = resultado.url.trim();
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

            let fonte = resultado
                .engine
                .filter(|valor| !valor.trim().is_empty())
                .or_else(|| resultado.engines.into_iter().next())
                .or_else(|| url.host_str().map(str::to_owned))
                .unwrap_or_else(|| "Fonte pública".to_owned());
            let titulo = resultado
                .title
                .filter(|valor| !valor.trim().is_empty())
                .unwrap_or_else(|| url_texto.to_owned());
            let parametro_utilizado = format!("{}: {}", parametro.tipo, parametro.valor);
            let (novo, pdf_salvo) = registrar_achado(
                &state,
                pessoa_id,
                &fonte,
                &parametro_utilizado,
                &titulo,
                resultado.content.as_deref(),
                url.as_str(),
                pdf,
            )
            .await?;
            resposta.novos_achados += usize::from(novo);
            resposta.pdfs_arquivados += usize::from(pdf_salvo);
        }
    }

    Ok(Json(resposta))
}

async fn registrar_achado(
    state: &AppState,
    pessoa_id: i64,
    fonte: &str,
    parametro_utilizado: &str,
    titulo: &str,
    snippet: Option<&str>,
    url_origem: &str,
    pdf: Option<PdfBaixado>,
) -> Result<(bool, bool), AppError> {
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
            (pessoa_id, fonte, parametro_utilizado, titulo_resultado, snippet, \
             url_origem, anexo_dossie_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(pessoa_id)
    .bind(truncar(fonte, 200))
    .bind(truncar(parametro_utilizado, 600))
    .bind(truncar(titulo, 1000))
    .bind(snippet.map(|valor| truncar(valor, 4000)))
    .bind(url_origem)
    .bind(anexo_id)
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
    !ip.is_private()
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
    if let Some(ipv4) = ip.to_ipv4_mapped() {
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

fn endpoint_searxng(base: &str) -> Result<Url, AppError> {
    let mut url = Url::parse(base).map_err(|_| {
        AppError::ServiceUnavailable("SEARXNG_URL não contém uma URL válida".to_owned())
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::ServiceUnavailable(
            "SEARXNG_URL deve usar HTTP ou HTTPS".to_owned(),
        ));
    }
    if !url.path().trim_end_matches('/').ends_with("/search") {
        let caminho = format!("{}/search", url.path().trim_end_matches('/'));
        url.set_path(&caminho);
    }
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn normalizar_parametro(input: ParametroBuscaInput) -> Result<(String, String, bool), AppError> {
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
    Ok((tipo, valor, input.ativo.unwrap_or(true)))
}

fn consulta_para(parametro: &ParametroBusca) -> String {
    if parametro.tipo == "TERMO" {
        return parametro.valor.clone();
    }
    format!("\"{}\"", parametro.valor.replace('"', " "))
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

#[derive(Debug, Deserialize)]
struct SearxResponse {
    #[serde(default)]
    results: Vec<SearxResult>,
}

#[derive(Debug, Deserialize)]
struct SearxResult {
    url: String,
    title: Option<String>,
    content: Option<String>,
    engine: Option<String>,
    #[serde(default)]
    engines: Vec<String>,
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
    use super::{consulta_para, endpoint_searxng, ipv4_publico};
    use crate::models::ParametroBusca;
    use std::net::Ipv4Addr;

    #[test]
    fn monta_consulta_com_aspas_exceto_termo() {
        let mut parametro = ParametroBusca {
            id: 1,
            pessoa_id: 1,
            tipo: "NOME".to_owned(),
            valor: "Maria Silva".to_owned(),
            ativo: true,
        };
        assert_eq!(consulta_para(&parametro), "\"Maria Silva\"");
        parametro.tipo = "TERMO".to_owned();
        assert_eq!(consulta_para(&parametro), "Maria Silva");
    }

    #[test]
    fn bloqueia_ips_nao_publicos() {
        assert!(!ipv4_publico(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!ipv4_publico(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(!ipv4_publico(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(ipv4_publico(Ipv4Addr::new(1, 1, 1, 1)));
    }

    #[test]
    fn completa_endpoint_searxng() {
        assert_eq!(
            endpoint_searxng("https://busca.exemplo.org/instancia")
                .unwrap()
                .as_str(),
            "https://busca.exemplo.org/instancia/search"
        );
    }
}
