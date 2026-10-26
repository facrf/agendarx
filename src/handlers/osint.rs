use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use reqwest::{Client, Url, redirect::Policy};
use sqlx::{QueryBuilder, Sqlite};
use tokio::net::lookup_host;

mod providers;
mod trabalhos;
pub use providers::OsintConfiguracao;
pub use trabalhos::{iniciar_rotina, interromper_trabalhos};

use providers::{PublicSearchProviders, SearchProvider};

use crate::{
    AppState,
    error::AppError,
    models::{
        HistoricoBuscaFiltro, HistoricoBuscaPaginadoResponse, HistoricoBuscaPublica,
        HistoricoBuscaResponse, ParametroBusca, ParametroBuscaInput, VarreduraResponse,
    },
};

const TIPOS_VALIDOS: [&str; 7] = [
    "NOME", "CPF", "CNPJ", "EMAIL", "TELEFONE", "TERMO", "PROCESSO",
];
const MAX_PARAMETROS_ATIVOS: usize = 50;
const ITENS_POR_PAGINA_VALIDOS: [i64; 4] = [0, 10, 50, 100];
const USER_AGENT: &str = concat!(
    "AgendarX-OSINT/",
    env!("CARGO_PKG_VERSION"),
    " (+arquivamento-publico)"
);

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route(
            "/configuracao",
            get(obter_configuracao).put(atualizar_configuracao),
        )
        .route(
            "/parametros/{pessoa_id}",
            get(listar_parametros).post(criar_parametro),
        )
        .route(
            "/parametros/{pessoa_id}/importar-perfil",
            post(importar_parametros_perfil),
        )
        .route(
            "/parametros/item/{id}",
            axum::routing::put(atualizar_parametro).delete(excluir_parametro),
        )
        .route("/varrer/{pessoa_id}", post(trabalhos::criar))
        .route("/trabalhos/pessoa/{pessoa_id}", get(trabalhos::listar))
        .route("/trabalhos/{id}/cancelar", post(trabalhos::cancelar))
        .route("/trabalhos/{id}/retomar", post(trabalhos::retomar))
        .route("/historico/{pessoa_id}", get(listar_historico))
        .route(
            "/historico/item/{id}",
            axum::routing::delete(excluir_item_historico),
        )
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct OsintConfigUpdateInput {
    pub searxng_url: Option<String>,
    pub cgu_api_key: Option<String>,
    #[serde(default)]
    pub limpar_cgu_api_key: bool,
    pub datajud_api_key: Option<String>,
    #[serde(default)]
    pub limpar_datajud_api_key: bool,
    pub openalex_api_key: Option<String>,
    #[serde(default)]
    pub limpar_openalex_api_key: bool,
    pub inlabs_username: Option<String>,
    pub inlabs_password: Option<String>,
    #[serde(default)]
    pub limpar_inlabs_credenciais: bool,
    pub inlabs_lookback_days: Option<u64>,
    pub pncp_base_url: Option<String>,
    pub pncp_tipos_documento: Option<String>,
    pub pncp_retroativo_dias: Option<u64>,
    pub cgu_base_url: Option<String>,
    pub tse_base_url: Option<String>,
    pub tse_anos: Option<String>,
    pub tse_abrangencias: Option<String>,
    pub tse_cargos: Option<String>,
    pub osint_timeout_seconds: Option<u64>,
    pub osint_max_results: Option<usize>,
    pub fontes_ativas: Option<Vec<String>>,
}

pub(crate) async fn carregar_osint_config(state: &AppState) -> Result<OsintConfiguracao, AppError> {
    let mut cfg = OsintConfiguracao::from_config(&state.config);
    let json_db: Option<String> =
        sqlx::query_scalar("SELECT config_json FROM osint_configuracao WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    if let Some(json_str) = json_db {
        cfg.aplicar_json_db(&json_str);
    }
    Ok(cfg)
}

fn mascarar_segredo(segredo: Option<&str>) -> Option<String> {
    let valor = segredo?.trim();
    if valor.is_empty() {
        return None;
    }
    let chars: Vec<char> = valor.chars().collect();
    if chars.len() <= 6 {
        return Some("••••••••".to_owned());
    }
    let inicio: String = chars.iter().take(4).collect();
    let fim: String = chars.iter().skip(chars.len() - 2).collect();
    Some(format!("{inicio}••••••••{fim}"))
}

fn serializar_config_resposta(cfg: &OsintConfiguracao) -> serde_json::Value {
    let fontes_status: Vec<serde_json::Value> = SearchProvider::ALL
        .into_iter()
        .map(|provider| {
            let tipos_compativeis: Vec<&str> = TIPOS_VALIDOS
                .into_iter()
                .filter(|tipo| provider.suporta_tipo(tipo))
                .collect();
            let requer_credencial = matches!(
                provider,
                SearchProvider::Cgu | SearchProvider::Searxng | SearchProvider::Inlabs
            );
            serde_json::json!({
                "provider": provider.as_str(),
                "label": provider.label(),
                "ativo": cfg.provider_ativo(provider),
                "configurado": cfg.provider_configurado(provider),
                "requer_credencial": requer_credencial,
                "tipos_compativeis": tipos_compativeis,
            })
        })
        .collect();

    serde_json::json!({
        "searxng_url": cfg.searxng_url.clone().unwrap_or_default(),
        "searxng_configurado": cfg.provider_configurado(SearchProvider::Searxng),
        "cgu_api_key_configurada": cfg.provider_configurado(SearchProvider::Cgu),
        "cgu_api_key_mascarada": mascarar_segredo(cfg.cgu_api_key.as_deref()),
        "datajud_api_key_configurada": cfg.datajud_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()),
        "datajud_api_key_mascarada": mascarar_segredo(cfg.datajud_api_key.as_deref()),
        "openalex_api_key_configurada": cfg.openalex_api_key.as_deref().is_some_and(|v| !v.trim().is_empty()),
        "openalex_api_key_mascarada": mascarar_segredo(cfg.openalex_api_key.as_deref()),
        "inlabs_username": cfg.inlabs_username.clone().unwrap_or_default(),
        "inlabs_configurado": cfg.provider_configurado(SearchProvider::Inlabs),
        "inlabs_lookback_days": cfg.inlabs_lookback_days,
        "pncp_base_url": cfg.pncp_base_url,
        "pncp_tipos_documento": cfg.pncp_tipos_documento,
        "pncp_retroativo_dias": cfg.pncp_retroativo_dias,
        "cgu_base_url": cfg.cgu_base_url,
        "tse_base_url": cfg.tse_base_url,
        "tse_anos": cfg.tse_anos,
        "tse_abrangencias": cfg.tse_abrangencias,
        "tse_cargos": cfg.tse_cargos,
        "osint_timeout_seconds": cfg.osint_timeout_seconds,
        "osint_max_results": cfg.osint_max_results,
        "fontes_ativas": cfg.fontes_ativas,
        "fontes_status": fontes_status,
    })
}

pub(crate) async fn obter_configuracao(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cfg = carregar_osint_config(&state).await?;
    Ok(Json(serializar_config_resposta(&cfg)))
}

pub(crate) async fn atualizar_configuracao(
    State(state): State<AppState>,
    Json(input): Json<OsintConfigUpdateInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut cfg = carregar_osint_config(&state).await?;

    if let Some(url) = input.searxng_url {
        let trimmed = url.trim().trim_end_matches('/').to_owned();
        if trimmed.is_empty() {
            cfg.searxng_url = None;
        } else {
            let parsed = Url::parse(&trimmed)
                .map_err(|_| AppError::BadRequest("URL do SearXNG inválida".into()))?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err(AppError::BadRequest(
                    "URL do SearXNG deve usar HTTP ou HTTPS".into(),
                ));
            }
            cfg.searxng_url = Some(trimmed);
        }
    }

    if input.limpar_cgu_api_key {
        cfg.cgu_api_key = None;
    } else if let Some(key) = input.cgu_api_key {
        let trimmed = key.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.cgu_api_key = Some(trimmed);
        }
    }

    if input.limpar_datajud_api_key {
        cfg.datajud_api_key = None;
    } else if let Some(key) = input.datajud_api_key {
        let trimmed = key.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.datajud_api_key = Some(trimmed);
        }
    }

    if input.limpar_openalex_api_key {
        cfg.openalex_api_key = None;
    } else if let Some(key) = input.openalex_api_key {
        let trimmed = key.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.openalex_api_key = Some(trimmed);
        }
    }

    if input.limpar_inlabs_credenciais {
        cfg.inlabs_username = None;
        cfg.inlabs_password = None;
    } else {
        if let Some(user) = input.inlabs_username {
            let trimmed = user.trim().to_owned();
            if trimmed.is_empty() {
                cfg.inlabs_username = None;
                cfg.inlabs_password = None;
            } else {
                cfg.inlabs_username = Some(trimmed);
            }
        }
        if let Some(pass) = input.inlabs_password {
            let trimmed = pass.trim().to_owned();
            if !trimmed.is_empty() {
                cfg.inlabs_password = Some(trimmed);
            }
        }
    }

    if let Some(days) = input.inlabs_lookback_days {
        if !(1..=7).contains(&days) {
            return Err(AppError::BadRequest(
                "Janela do INLABS deve estar entre 1 e 7 dias".into(),
            ));
        }
        cfg.inlabs_lookback_days = days;
    }

    if let Some(url) = input.pncp_base_url {
        let trimmed = url.trim().trim_end_matches('/').to_owned();
        if !trimmed.is_empty() {
            cfg.pncp_base_url = trimmed;
        }
    }
    if let Some(tipos) = input.pncp_tipos_documento {
        let trimmed = tipos.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.pncp_tipos_documento = trimmed;
        }
    }
    if let Some(dias) = input.pncp_retroativo_dias {
        if !(1..=3650).contains(&dias) {
            return Err(AppError::BadRequest(
                "Janela retroativa do PNCP deve estar entre 1 e 3650 dias".into(),
            ));
        }
        cfg.pncp_retroativo_dias = dias;
    }
    if let Some(url) = input.cgu_base_url {
        let trimmed = url.trim().trim_end_matches('/').to_owned();
        if !trimmed.is_empty() {
            cfg.cgu_base_url = trimmed;
        }
    }
    if let Some(url) = input.tse_base_url {
        let trimmed = url.trim().trim_end_matches('/').to_owned();
        if !trimmed.is_empty() {
            cfg.tse_base_url = trimmed;
        }
    }
    if let Some(anos) = input.tse_anos {
        let trimmed = anos.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.tse_anos = trimmed;
        }
    }
    if let Some(abr) = input.tse_abrangencias {
        let trimmed = abr.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.tse_abrangencias = trimmed;
        }
    }
    if let Some(cargos) = input.tse_cargos {
        let trimmed = cargos.trim().to_owned();
        if !trimmed.is_empty() {
            cfg.tse_cargos = trimmed;
        }
    }
    if let Some(timeout) = input.osint_timeout_seconds {
        if !(5..=120).contains(&timeout) {
            return Err(AppError::BadRequest(
                "Tempo limite OSINT deve estar entre 5 e 120 segundos".into(),
            ));
        }
        cfg.osint_timeout_seconds = timeout;
    }
    if let Some(max) = input.osint_max_results {
        if !(1..=100).contains(&max) {
            return Err(AppError::BadRequest(
                "Máximo de resultados por fonte deve estar entre 1 e 100".into(),
            ));
        }
        cfg.osint_max_results = max;
    }
    if let Some(fontes) = input.fontes_ativas {
        let mut validas = Vec::new();
        for item in fontes {
            let provider = SearchProvider::parse(&item).map_err(AppError::BadRequest)?;
            let code = provider.as_str().to_owned();
            if !validas.contains(&code) {
                validas.push(code);
            }
        }
        if validas.is_empty() {
            return Err(AppError::BadRequest(
                "Mantenha ao menos uma fonte de pesquisa pública ativa".into(),
            ));
        }
        cfg.fontes_ativas = validas;
    }

    let json_str = serde_json::to_string(&cfg).map_err(|e| AppError::interno(e.to_string()))?;
    sqlx::query(
        "INSERT INTO osint_configuracao (id, config_json, atualizado_em) VALUES (1, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json, atualizado_em = CURRENT_TIMESTAMP",
    )
    .bind(json_str)
    .execute(&state.pool)
    .await?;

    Ok(Json(serializar_config_resposta(&cfg)))
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

async fn importar_parametros_perfil(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let (nome, _descricao): (String, Option<String>) =
        sqlx::query_as("SELECT nome, descricao FROM pessoa WHERE id = ?")
            .bind(pessoa_id)
            .fetch_one(&state.pool)
            .await?;
    let contatos: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.nome_tipo, c.valor FROM contato c \
         JOIN tipo_meio_contato t ON t.id = c.tipo_contato_id \
         WHERE c.pessoa_id = ? ORDER BY c.id",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;

    let mut candidatos: Vec<(String, String)> = Vec::new();
    if !nome.trim().is_empty() {
        candidatos.push(("NOME".to_owned(), nome.trim().to_owned()));
    }
    for (nome_tipo, valor) in contatos {
        let val = valor.trim();
        if val.is_empty() {
            continue;
        }
        let digitos: String = val.chars().filter(char::is_ascii_digit).collect();
        let tipo_upper = nome_tipo.to_ascii_uppercase();
        if tipo_upper.contains("CPF")
            || (digitos.len() == 11
                && !tipo_upper.contains("TEL")
                && !tipo_upper.contains("CEL")
                && !tipo_upper.contains("WHATS")
                && !tipo_upper.contains("FONE"))
        {
            candidatos.push(("CPF".to_owned(), val.to_owned()));
        } else if tipo_upper.contains("CNPJ") || digitos.len() == 14 {
            candidatos.push(("CNPJ".to_owned(), val.to_owned()));
        } else if tipo_upper.contains("MAIL") || val.contains('@') {
            candidatos.push(("EMAIL".to_owned(), val.to_owned()));
        } else if (tipo_upper.contains("TEL")
            || tipo_upper.contains("CEL")
            || tipo_upper.contains("WHATS")
            || tipo_upper.contains("FONE"))
            && (7..=15).contains(&digitos.len())
        {
            candidatos.push(("TELEFONE".to_owned(), val.to_owned()));
        } else if tipo_upper.contains("PROCESSO") {
            candidatos.push(("PROCESSO".to_owned(), val.to_owned()));
        }
    }

    let mut importados = 0usize;
    for (tipo_cand, valor_cand) in candidatos {
        let Ok((tipo_norm, valor_norm, provider_norm, _)) =
            normalizar_parametro(ParametroBuscaInput {
                tipo: tipo_cand,
                valor: valor_cand,
                provider: "COMPATIVEIS".to_owned(),
                ativo: Some(true),
            })
        else {
            continue;
        };
        let ja_existe: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM parametro_busca WHERE pessoa_id = ? AND tipo = ? AND lower(trim(valor)) = lower(trim(?)))",
        )
        .bind(pessoa_id)
        .bind(&tipo_norm)
        .bind(&valor_norm)
        .fetch_one(&state.pool)
        .await?;
        if ja_existe {
            continue;
        }
        let res = sqlx::query(
            "INSERT OR IGNORE INTO parametro_busca (pessoa_id, tipo, valor, provider, ativo) VALUES (?, ?, ?, ?, 1)",
        )
        .bind(pessoa_id)
        .bind(&tipo_norm)
        .bind(&valor_norm)
        .bind(&provider_norm)
        .execute(&state.pool)
        .await?;
        if res.rows_affected() > 0 {
            importados += 1;
        }
    }

    let parametros = sqlx::query_as::<_, ParametroBusca>(
        "SELECT id, pessoa_id, tipo, valor, provider, ativo FROM parametro_busca \
         WHERE pessoa_id = ? ORDER BY ativo DESC, provider, tipo, valor",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({
        "importados": importados,
        "parametros": parametros,
    })))
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
    Query(filtro): Query<HistoricoBuscaFiltro>,
) -> Result<Json<HistoricoBuscaPaginadoResponse>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let pagina = filtro.pagina.unwrap_or(1);
    if pagina < 1 {
        return Err(AppError::BadRequest(
            "pagina deve ser maior ou igual a 1".to_owned(),
        ));
    }
    let por_pagina = filtro.por_pagina.unwrap_or(10);
    if !ITENS_POR_PAGINA_VALIDOS.contains(&por_pagina) {
        return Err(AppError::BadRequest(
            "por_pagina deve ser 10, 50, 100 ou 0 para todos".to_owned(),
        ));
    }
    let busca = filtro
        .busca
        .map(|valor| valor.trim().to_owned())
        .filter(|valor| !valor.is_empty());
    if busca
        .as_ref()
        .is_some_and(|valor| valor.chars().count() > 200)
    {
        return Err(AppError::BadRequest(
            "a busca deve possuir no máximo 200 caracteres".to_owned(),
        ));
    }
    let padrao_busca = busca.as_deref().map(padrao_like);

    let mut consulta_total = QueryBuilder::<Sqlite>::new(
        "SELECT COUNT(*) FROM historico_busca_publica WHERE pessoa_id = ",
    );
    consulta_total.push_bind(pessoa_id);
    adicionar_filtro_busca(&mut consulta_total, padrao_busca.as_deref());
    let total = consulta_total
        .build_query_scalar::<i64>()
        .fetch_one(&state.pool)
        .await?;

    let pagina_efetiva = if por_pagina == 0 { 1 } else { pagina };
    let mut consulta = QueryBuilder::<Sqlite>::new(
        "SELECT id, pessoa_id, fonte, provider, parametro_utilizado, titulo_resultado, snippet, \
         url_origem, anexo_dossie_id, data_publicacao, detalhes, data_captura \
         FROM historico_busca_publica WHERE pessoa_id = ",
    );
    consulta.push_bind(pessoa_id);
    adicionar_filtro_busca(&mut consulta, padrao_busca.as_deref());
    consulta.push(" ORDER BY data_captura DESC, id DESC");
    if por_pagina > 0 {
        consulta
            .push(" LIMIT ")
            .push_bind(por_pagina)
            .push(" OFFSET ")
            .push_bind((pagina_efetiva - 1).saturating_mul(por_pagina));
    }
    let itens = consulta
        .build_query_as::<HistoricoBuscaPublica>()
        .fetch_all(&state.pool)
        .await?;
    let total_paginas = if por_pagina == 0 {
        i64::from(total > 0)
    } else {
        (total + por_pagina - 1) / por_pagina
    };

    Ok(Json(HistoricoBuscaPaginadoResponse {
        itens: itens.into_iter().map(resposta_historico).collect(),
        total,
        pagina: pagina_efetiva,
        por_pagina,
        total_paginas,
    }))
}

async fn excluir_item_historico(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM historico_busca_publica WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("achado da linha do tempo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn adicionar_filtro_busca<'a>(consulta: &mut QueryBuilder<'a, Sqlite>, padrao: Option<&'a str>) {
    let Some(padrao) = padrao else {
        return;
    };
    consulta.push(" AND (titulo_resultado LIKE ");
    consulta.push_bind(padrao);
    for campo in [
        "snippet",
        "fonte",
        "provider",
        "parametro_utilizado",
        "detalhes",
        "url_origem",
    ] {
        consulta.push(" ESCAPE '\\' OR ").push(campo).push(" LIKE ");
        consulta.push_bind(padrao);
    }
    consulta.push(" ESCAPE '\\')");
}

fn padrao_like(valor: &str) -> String {
    let escapado = valor
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escapado}%")
}

fn resposta_historico(item: HistoricoBuscaPublica) -> HistoricoBuscaResponse {
    HistoricoBuscaResponse {
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
    }
}

async fn executar_parametros(
    state: AppState,
    pessoa_id: i64,
    parametros: Vec<ParametroBusca>,
    contexto: &trabalhos::Contexto,
    providers: &mut PublicSearchProviders,
) -> Result<(VarreduraResponse, HashSet<String>), AppError> {
    let parametros = if parametros
        .iter()
        .any(|p| p.provider.eq_ignore_ascii_case("COMPATIVEIS"))
    {
        let cfg = carregar_osint_config(&state).await?;
        cfg.expandir_parametros(parametros, false)
    } else {
        parametros
    };
    let inicio_varredura = Instant::now();
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
        let execution = contexto
            .aguardar(&state, providers.search(provider, &parametro))
            .await?;
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
            let leitura = contexto.validar(&state).await?;
            let ja_existe: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM historico_busca_publica \
                 WHERE pessoa_id = ? AND url_origem = ?)",
            )
            .bind(pessoa_id)
            .bind(url_texto)
            .fetch_one(&state.pool)
            .await?;
            drop(leitura);
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
                match contexto
                    .aguardar(
                        &state,
                        tokio::time::timeout(
                            Duration::from_secs(state.config.osint_timeout_seconds),
                            baixar_pdf(&state, &url),
                        ),
                    )
                    .await?
                    .unwrap_or_else(|_| Err("tempo limite do download excedido".into()))
                {
                    Ok(arquivo) => pdf = Some(arquivo),
                    Err(erro) => resposta.avisos.push(format!(
                        "PDF não arquivado em {}: {erro}",
                        resumir_url(&url)
                    )),
                }
            }

            let parametro_utilizado = format!("{}: {}", parametro.tipo, parametro.valor);
            let _leitura = contexto.validar(&state).await?;
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

    Ok((resposta, fontes_indisponiveis))
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
    let provider_str = if matches!(
        input.provider.trim().to_ascii_uppercase().as_str(),
        "COMPATIVEIS" | "AUTO"
    ) {
        "COMPATIVEIS".to_owned()
    } else {
        let provider = SearchProvider::parse(&input.provider).map_err(AppError::BadRequest)?;
        if provider == SearchProvider::Datajud {
            if tipo != "PROCESSO" {
                return Err(AppError::BadRequest(
                    "DataJud público exige o número do processo; não pesquisa nomes ou CPF das partes"
                        .into(),
                ));
            }
            providers::datajud_query(&valor).map_err(AppError::BadRequest)?;
        } else if !provider.suporta_tipo(&tipo) {
            return Err(AppError::BadRequest(format!(
                "{} não aceita parâmetros do tipo {}; use o modo Automático (Compatíveis) ou selecione um tipo compatível",
                provider.label(),
                tipo
            )));
        }
        provider.as_str().to_owned()
    };
    Ok((tipo, valor, provider_str, input.ativo.unwrap_or(true)))
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
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ? AND excluida_em IS NULL)",
    )
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
    use super::{
        classificar_varredura, ipv4_publico, ipv6_publico, normalizar_parametro, padrao_like,
    };
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
        for provider in [
            "COMPATIVEIS",
            "SEARXNG",
            "QUERIDO_DIARIO",
            "INLABS",
            "OPENALEX",
            "PNCP",
        ] {
            let input: ParametroBuscaInput = serde_json::from_value(serde_json::json!({
                "tipo": "TERMO",
                "valor": "pesquisa",
                "provider": provider
            }))
            .unwrap();
            let (_, _, normalized, _) = normalizar_parametro(input).unwrap();
            assert_eq!(normalized, provider);
        }
        for provider in ["CGU", "TSE", "PNCP", "COMPATIVEIS"] {
            let input: ParametroBuscaInput = serde_json::from_value(serde_json::json!({
                "tipo": "CNPJ",
                "valor": "00.394.460/0001-41",
                "provider": provider
            }))
            .unwrap();
            let (_, _, normalized, _) = normalizar_parametro(input).unwrap();
            assert_eq!(normalized, provider);
        }
    }

    #[test]
    fn datajud_requires_process_number() {
        for (tipo, valor, valid) in [
            ("PROCESSO", "0000832-35.2018.4.01.3202", true),
            ("NOME", "Maria Silva", false),
            ("PROCESSO", "123", false),
        ] {
            let input = serde_json::from_value(
                serde_json::json!({"tipo":tipo,"valor":valor,"provider":"DATAJUD"}),
            )
            .unwrap();
            assert_eq!(normalizar_parametro(input).is_ok(), valid);
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
    async fn datajud_migration_preserves_existing_searches_and_history() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for migration in [
            include_str!("../../migrations/0001_schema_inicial.sql"),
            include_str!("../../migrations/0003_busca_publica.sql"),
            include_str!("../../migrations/0009_fontes_pesquisa_publica.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql("INSERT INTO pessoa (id, nome) VALUES (1, 'Teste'); INSERT INTO parametro_busca (id, pessoa_id, tipo, valor, provider) VALUES (7, 1, 'TERMO', 'legado', 'OPENALEX'); INSERT INTO historico_busca_publica (id, pessoa_id, fonte, parametro_utilizado, titulo_resultado, url_origem, provider, detalhes) VALUES (9, 1, 'OpenAlex', 'legado', 'Resultado', 'https://example.org', 'OPENALEX', 'Preservado');").execute(&pool).await.unwrap();
        sqlx::raw_sql(include_str!("../../migrations/0016_datajud.sql"))
            .execute(&pool)
            .await
            .unwrap();
        let old: String = sqlx::query_scalar(
            "SELECT detalhes FROM historico_busca_publica WHERE id = 9 AND provider = 'OPENALEX'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(old, "Preservado");
        let parameter: String =
            sqlx::query_scalar("SELECT valor FROM parametro_busca WHERE id = 7")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(parameter, "legado");
        sqlx::query("INSERT INTO parametro_busca (pessoa_id, tipo, valor, provider) VALUES (1, 'PROCESSO', '00008323520184013202', 'DATAJUD')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO historico_busca_publica (pessoa_id, fonte, parametro_utilizado, titulo_resultado, url_origem, provider) VALUES (1, 'CNJ', 'processo', 'Processo', 'https://example.org/datajud', 'DATAJUD')").execute(&pool).await.unwrap();
        let violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(violations.is_empty());
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

        for provider in ["SEARXNG", "QUERIDO_DIARIO", "INLABS", "OPENALEX", "DATAJUD"] {
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
        assert_eq!(saved, 5);

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
    fn busca_no_historico_escapa_curingas_sql() {
        assert_eq!(padrao_like(r"50%_\ concluído"), r"%50\%\_\\ concluído%");
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
