use std::{
    collections::{BTreeSet, HashSet},
    io::{Cursor, Read},
    time::Duration,
};

use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use quick_xml::{Reader, events::Event};
use reqwest::{Client, StatusCode, Url, header, redirect::Policy};
use serde::Deserialize;
use zip::ZipArchive;

use crate::{config::Config, models::ParametroBusca};

const USER_AGENT: &str = "AgendarX-OSINT/0.4 (+pesquisa-publica)";
const QUERIDO_DIARIO_URL: &str = "https://api.queridodiario.ok.org.br/gazettes";
const OPENALEX_URL: &str = "https://api.openalex.org/works";
const INLABS_LOGIN_URL: &str = "https://inlabs.in.gov.br/logar.php";
const INLABS_DOWNLOAD_URL: &str = "https://inlabs.in.gov.br/index.php";
const INLABS_SECTIONS: [&str; 6] = ["DO1", "DO2", "DO3", "DO1E", "DO2E", "DO3E"];
const INLABS_MAX_ZIP_BYTES: usize = 100 * 1024 * 1024;
const INLABS_MAX_XML_BYTES: u64 = 4 * 1024 * 1024;
const INLABS_MAX_FILES_PER_ZIP: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchProvider {
    Searxng,
    QueridoDiario,
    Inlabs,
    OpenAlex,
}

impl SearchProvider {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_uppercase().as_str() {
            "" | "SEARXNG" => Ok(Self::Searxng),
            "QUERIDO_DIARIO" => Ok(Self::QueridoDiario),
            "INLABS" => Ok(Self::Inlabs),
            "OPENALEX" => Ok(Self::OpenAlex),
            _ => Err("fonte inválida; use SEARXNG, QUERIDO_DIARIO, INLABS ou OPENALEX".to_owned()),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Searxng => "SEARXNG",
            Self::QueridoDiario => "QUERIDO_DIARIO",
            Self::Inlabs => "INLABS",
            Self::OpenAlex => "OPENALEX",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Searxng => "SearXNG",
            Self::QueridoDiario => "Querido Diário",
            Self::Inlabs => "INLABS / DOU",
            Self::OpenAlex => "OpenAlex",
        }
    }
}

#[derive(Debug)]
pub struct PublicSearchResult {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub published_at: Option<String>,
    pub source: String,
    pub details: Option<String>,
}

#[derive(Debug, Default)]
pub struct SearchExecution {
    pub results: Vec<PublicSearchResult>,
    pub warnings: Vec<String>,
    pub unavailable_sources: BTreeSet<String>,
    pub requests_executed: usize,
    pub inconclusive: bool,
    pub degraded: bool,
}

#[derive(Debug, Clone)]
struct ProviderFailure {
    user_message: String,
}

impl ProviderFailure {
    fn new(message: impl Into<String>) -> Self {
        Self {
            user_message: message.into(),
        }
    }
}

enum InlabsCache {
    Empty,
    Ready(Vec<InlabsDocument>),
    Failed(ProviderFailure),
}

pub struct PublicSearchProviders {
    client: Client,
    searxng_endpoint: Option<Url>,
    searxng_configuration_error: Option<String>,
    openalex_api_key: Option<String>,
    inlabs_username: Option<String>,
    inlabs_password: Option<String>,
    inlabs_lookback_days: u64,
    max_results: usize,
    inlabs_cache: InlabsCache,
}

impl PublicSearchProviders {
    pub fn new(config: &Config) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.osint_timeout_seconds))
            .redirect(Policy::limited(3))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| format!("falha ao configurar cliente HTTP: {error}"))?;
        let (searxng_endpoint, searxng_configuration_error) = match config.searxng_url.as_deref() {
            Some(base) => match endpoint_searxng(base) {
                Ok(url) => (Some(url), None),
                Err(error) => (None, Some(error)),
            },
            None => (
                None,
                Some("SearXNG não configurado. Configure SEARXNG_URL no servidor.".to_owned()),
            ),
        };

        Ok(Self {
            client,
            searxng_endpoint,
            searxng_configuration_error,
            openalex_api_key: config.openalex_api_key.clone(),
            inlabs_username: config.inlabs_username.clone(),
            inlabs_password: config.inlabs_password.clone(),
            inlabs_lookback_days: config.inlabs_lookback_days,
            max_results: config.osint_max_results,
            inlabs_cache: InlabsCache::Empty,
        })
    }

    pub async fn search(
        &mut self,
        provider: SearchProvider,
        parameter: &ParametroBusca,
    ) -> SearchExecution {
        let outcome = match provider {
            SearchProvider::Searxng => self.search_searxng(parameter).await,
            SearchProvider::QueridoDiario => self.search_querido_diario(parameter).await,
            SearchProvider::Inlabs => self.search_inlabs(parameter).await,
            SearchProvider::OpenAlex => self.search_openalex(parameter).await,
        };

        match outcome {
            Ok(execution) => execution,
            Err(error) => {
                tracing::warn!(
                    provider = provider.as_str(),
                    parameter_type = %parameter.tipo,
                    error = %error.user_message,
                    "falha isolada em provider da Pesquisa Pública"
                );
                let mut unavailable_sources = BTreeSet::new();
                unavailable_sources.insert(provider.label().to_owned());
                SearchExecution {
                    warnings: vec![format!("{}: {}", provider.label(), error.user_message)],
                    unavailable_sources,
                    requests_executed: 1,
                    inconclusive: true,
                    degraded: true,
                    ..SearchExecution::default()
                }
            }
        }
    }

    async fn search_searxng(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        let endpoint = self.searxng_endpoint.clone().ok_or_else(|| {
            ProviderFailure::new(
                self.searxng_configuration_error
                    .as_deref()
                    .unwrap_or("SearXNG não configurado"),
            )
        })?;
        let queries = queries_for_searxng(parameter);
        let mut execution = SearchExecution::default();
        let mut received_valid_json = false;
        let mut failed_request = false;
        let mut urls = HashSet::new();
        let mut unavailable_details = BTreeSet::new();

        for query in queries {
            if execution.results.len() >= self.max_results {
                break;
            }
            execution.requests_executed += 1;
            let response = match self
                .client
                .get(endpoint.clone())
                .header(header::ACCEPT, "application/json")
                .query(&[
                    ("q", query.as_str()),
                    ("format", "json"),
                    ("language", "pt-BR"),
                    ("safesearch", "0"),
                ])
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => response,
                Ok(response) => {
                    failed_request = true;
                    execution
                        .warnings
                        .push(searxng_status_message(&parameter.tipo, response.status()));
                    continue;
                }
                Err(error) => {
                    failed_request = true;
                    execution.warnings.push(format!(
                        "{}: não foi possível consultar o SearXNG ({})",
                        parameter.tipo,
                        request_error_message(&error)
                    ));
                    continue;
                }
            };

            let data: SearxResponse = match response.json().await {
                Ok(data) => data,
                Err(error) => {
                    failed_request = true;
                    execution.warnings.push(format!(
                        "{}: resposta JSON inválida do SearXNG ({})",
                        parameter.tipo,
                        request_error_message(&error)
                    ));
                    continue;
                }
            };
            received_valid_json = true;
            for (source, reason) in unavailable_sources_from_searxng(&data) {
                execution.unavailable_sources.insert(source.clone());
                unavailable_details.insert((source, reason));
            }
            for result in data.results {
                let url = result.url.trim();
                if url.is_empty() || !urls.insert(url.to_owned()) {
                    continue;
                }
                let source = result
                    .engine
                    .filter(|value| !value.trim().is_empty())
                    .or_else(|| result.engines.into_iter().next())
                    .or_else(|| {
                        Url::parse(url)
                            .ok()
                            .and_then(|parsed| parsed.host_str().map(str::to_owned))
                    })
                    .unwrap_or_else(|| "Fonte pública".to_owned());
                execution.results.push(PublicSearchResult {
                    title: result
                        .title
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| url.to_owned()),
                    url: url.to_owned(),
                    description: result.content,
                    published_at: None,
                    source,
                    details: None,
                });
                if execution.results.len() >= self.max_results {
                    break;
                }
            }
        }

        if !unavailable_details.is_empty() {
            execution.warnings.push(unavailable_sources_message(
                &parameter.tipo,
                &unavailable_details,
            ));
        }
        execution.inconclusive = !received_valid_json
            || (execution.results.is_empty() && !unavailable_details.is_empty());
        execution.degraded =
            execution.inconclusive || failed_request || !unavailable_details.is_empty();
        Ok(execution)
    }

    async fn search_querido_diario(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        let response = self
            .client
            .get(QUERIDO_DIARIO_URL)
            .header(header::ACCEPT, "application/json")
            .query(&[
                ("querystring", provider_query(parameter)),
                ("excerpt_size", "500"),
                ("number_of_excerpts", "3"),
                ("size", &self.max_results.to_string()),
            ])
            .send()
            .await
            .map_err(|error| provider_request_error("Querido Diário", error))?;
        ensure_success("Querido Diário", response.status())?;
        let data: QueridoDiarioResponse = response.json().await.map_err(|error| {
            tracing::warn!(
                provider = "QUERIDO_DIARIO",
                decodificacao = error.is_decode(),
                status = ?error.status(),
                "JSON inválido do provider"
            );
            ProviderFailure::new("a API retornou uma resposta inválida")
        })?;
        Ok(SearchExecution {
            results: normalize_querido_diario(data, self.max_results),
            requests_executed: 1,
            ..SearchExecution::default()
        })
    }

    async fn search_openalex(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        let mut request = self
            .client
            .get(OPENALEX_URL)
            .header(header::ACCEPT, "application/json")
            .query(&[
                ("search", provider_query(parameter)),
                ("per-page", &self.max_results.to_string()),
                ("select", "id,doi,display_name,publication_date,publication_year,cited_by_count,authorships,primary_location"),
            ]);
        if let Some(api_key) = self.openalex_api_key.as_deref() {
            request = request.bearer_auth(api_key);
        }
        let response = request
            .send()
            .await
            .map_err(|error| provider_request_error("OpenAlex", error))?;
        ensure_success("OpenAlex", response.status())?;
        let data: OpenAlexResponse = response.json().await.map_err(|error| {
            tracing::warn!(
                provider = "OPENALEX",
                decodificacao = error.is_decode(),
                status = ?error.status(),
                "JSON inválido do provider"
            );
            ProviderFailure::new("a API retornou uma resposta inválida")
        })?;
        Ok(SearchExecution {
            results: normalize_openalex(data, self.max_results),
            requests_executed: 1,
            ..SearchExecution::default()
        })
    }

    async fn search_inlabs(
        &mut self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        if matches!(self.inlabs_cache, InlabsCache::Empty) {
            self.inlabs_cache = match self.load_inlabs_documents().await {
                Ok(documents) => InlabsCache::Ready(documents),
                Err(error) => InlabsCache::Failed(error),
            };
        }

        match &self.inlabs_cache {
            InlabsCache::Ready(documents) => Ok(SearchExecution {
                results: search_inlabs_documents(documents, parameter, self.max_results),
                requests_executed: 1,
                ..SearchExecution::default()
            }),
            InlabsCache::Failed(error) => Err(error.clone()),
            InlabsCache::Empty => unreachable!("cache do INLABS foi inicializado"),
        }
    }

    async fn load_inlabs_documents(&self) -> Result<Vec<InlabsDocument>, ProviderFailure> {
        let username = self.inlabs_username.as_deref().ok_or_else(|| {
            ProviderFailure::new(
                "INLABS não configurado. Configure INLABS_USERNAME e INLABS_PASSWORD.",
            )
        })?;
        let password = self.inlabs_password.as_deref().ok_or_else(|| {
            ProviderFailure::new(
                "INLABS não configurado. Configure INLABS_USERNAME e INLABS_PASSWORD.",
            )
        })?;

        let login_response = self
            .client
            .post(INLABS_LOGIN_URL)
            .form(&[("email", username), ("password", password)])
            .send()
            .await
            .map_err(|error| provider_request_error("INLABS / DOU", error))?;
        ensure_success("INLABS / DOU", login_response.status())?;
        let session_cookie = extract_inlabs_cookie(login_response.headers()).ok_or_else(|| {
            ProviderFailure::new(
                "não foi possível autenticar no INLABS; verifique as credenciais configuradas",
            )
        })?;

        let mut documents = Vec::new();
        let today = Local::now().date_naive();
        for offset in 0..self.inlabs_lookback_days {
            let date = today - ChronoDuration::days(offset as i64);
            let date_text = date.format("%Y-%m-%d").to_string();
            for section in INLABS_SECTIONS {
                let filename = format!("{date_text}-{section}.zip");
                let response = self
                    .client
                    .get(INLABS_DOWNLOAD_URL)
                    .header(
                        header::COOKIE,
                        format!("inlabs_session_cookie={session_cookie}"),
                    )
                    .header("origem", "736372697074")
                    .query(&[("p", date_text.as_str()), ("dl", filename.as_str())])
                    .send()
                    .await
                    .map_err(|error| provider_request_error("INLABS / DOU", error))?;
                if response.status() == StatusCode::NOT_FOUND {
                    continue;
                }
                ensure_success("INLABS / DOU", response.status())?;
                let archive = read_limited_body(response, INLABS_MAX_ZIP_BYTES).await?;
                if !archive.starts_with(b"PK") {
                    tracing::warn!(provider = "INLABS", section, %date_text, "resposta não contém ZIP válido");
                    continue;
                }
                let parsed = tokio::task::spawn_blocking(move || parse_inlabs_zip(archive))
                    .await
                    .map_err(|error| {
                        tracing::warn!(provider = "INLABS", error = %error, "processamento de ZIP falhou");
                        ProviderFailure::new("não foi possível processar um arquivo do DOU")
                    })??;
                documents.extend(parsed);
            }
        }
        Ok(documents)
    }
}

fn provider_query(parameter: &ParametroBusca) -> &str {
    parameter.valor.trim()
}

fn endpoint_searxng(base: &str) -> Result<Url, String> {
    let mut url =
        Url::parse(base).map_err(|_| "SEARXNG_URL não contém uma URL válida".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("SEARXNG_URL deve usar HTTP ou HTTPS".to_owned());
    }
    if !url.path().trim_end_matches('/').ends_with("/search") {
        let path = format!("{}/search", url.path().trim_end_matches('/'));
        url.set_path(&path);
    }
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn ensure_success(provider: &str, status: StatusCode) -> Result<(), ProviderFailure> {
    if status.is_success() {
        return Ok(());
    }
    tracing::warn!(provider, %status, "provider respondeu com erro HTTP");
    let message = match status {
        StatusCode::TOO_MANY_REQUESTS => {
            "limite de consultas atingido; tente novamente mais tarde".to_owned()
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "acesso recusado; verifique a configuração do serviço".to_owned()
        }
        value if value.is_server_error() => "serviço temporariamente indisponível".to_owned(),
        _ => format!("o serviço recusou a consulta (HTTP {status})"),
    };
    Err(ProviderFailure::new(message))
}

fn provider_request_error(provider: &str, error: reqwest::Error) -> ProviderFailure {
    tracing::warn!(
        provider,
        timeout = error.is_timeout(),
        conexao = error.is_connect(),
        decodificacao = error.is_decode(),
        status = ?error.status(),
        "requisição ao provider falhou"
    );
    ProviderFailure::new(request_error_message(&error))
}

fn request_error_message(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "tempo limite da consulta excedido"
    } else if error.is_connect() {
        "não foi possível conectar ao serviço"
    } else if error.is_decode() {
        "o serviço retornou dados inválidos"
    } else {
        "falha temporária ao consultar o serviço"
    }
}

async fn read_limited_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, ProviderFailure> {
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(ProviderFailure::new(
            "arquivo do INLABS excede o limite de segurança",
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        tracing::warn!(provider = "INLABS", error = %error, "download de ZIP interrompido");
        ProviderFailure::new("não foi possível concluir o download do DOU")
    })? {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(ProviderFailure::new(
                "arquivo do INLABS excede o limite de segurança",
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn extract_inlabs_cookie(headers: &header::HeaderMap) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie| {
            cookie.split(';').find_map(|part| {
                part.trim()
                    .strip_prefix("inlabs_session_cookie=")
                    .map(str::to_owned)
                    .filter(|value| !value.is_empty())
            })
        })
}

#[derive(Debug, Deserialize)]
struct SearxResponse {
    #[serde(default)]
    results: Vec<SearxResult>,
    #[serde(default)]
    unresponsive_engines: Vec<Vec<String>>,
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

fn searxng_status_message(parameter_type: &str, status: StatusCode) -> String {
    if status == StatusCode::FORBIDDEN {
        return format!(
            "{parameter_type}: o SearXNG recusou a API JSON (HTTP 403). Habilite 'json' em search.formats no settings.yml e, se o limiter estiver ativo, configure corretamente os cabeçalhos do proxy"
        );
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return format!("{parameter_type}: limite de consultas do SearXNG atingido");
    }
    format!("{parameter_type}: o SearXNG respondeu com HTTP {status}")
}

fn unavailable_sources_from_searxng(data: &SearxResponse) -> Vec<(String, String)> {
    data.unresponsive_engines
        .iter()
        .filter_map(|item| {
            let source = item.first()?.trim();
            if source.is_empty() {
                return None;
            }
            let reason = item
                .get(1)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .unwrap_or("indisponível");
            Some((source.to_owned(), reason.to_owned()))
        })
        .collect()
}

fn unavailable_sources_message(
    parameter_type: &str,
    sources: &BTreeSet<(String, String)>,
) -> String {
    let details = sources
        .iter()
        .map(|(source, reason)| format!("{source} ({})", translate_source_reason(reason)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{parameter_type}: fontes temporariamente indisponíveis: {details}")
}

fn translate_source_reason(reason: &str) -> String {
    let normalized = reason.to_ascii_lowercase();
    if normalized.contains("captcha") {
        "CAPTCHA".to_owned()
    } else if normalized.contains("too many requests") || normalized.contains("rate limit") {
        "limite de requisições".to_owned()
    } else if normalized.contains("timeout") {
        "tempo esgotado".to_owned()
    } else if normalized.contains("suspended") {
        "temporariamente suspensa".to_owned()
    } else {
        truncate(reason, 120)
    }
}

fn queries_for_searxng(parameter: &ParametroBusca) -> Vec<String> {
    let value = parameter.valor.replace('"', " ").trim().to_owned();
    let mut values = Vec::new();
    push_unique(&mut values, value.clone());
    match parameter.tipo.as_str() {
        "CPF" | "CNPJ" | "TELEFONE" => {
            let digits: String = value.chars().filter(char::is_ascii_digit).collect();
            if !digits.is_empty() {
                push_unique(&mut values, digits.clone());
                if let Some(formatted) = format_identifier(&parameter.tipo, &digits) {
                    push_unique(&mut values, formatted);
                }
            }
        }
        _ => {}
    }
    if parameter.tipo == "TERMO" {
        return values;
    }
    if parameter.tipo == "NOME" {
        let mut queries = vec![format!("\"{value}\"")];
        push_unique(&mut queries, value);
        return queries;
    }
    values
        .into_iter()
        .map(|value| format!("\"{value}\""))
        .collect()
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.contains(&value) {
        values.push(value);
    }
}

fn format_identifier(parameter_type: &str, digits: &str) -> Option<String> {
    match (parameter_type, digits.len()) {
        ("CPF", 11) => Some(format!(
            "{}.{}.{}-{}",
            &digits[0..3],
            &digits[3..6],
            &digits[6..9],
            &digits[9..11]
        )),
        ("CNPJ", 14) => Some(format!(
            "{}.{}.{}/{}-{}",
            &digits[0..2],
            &digits[2..5],
            &digits[5..8],
            &digits[8..12],
            &digits[12..14]
        )),
        ("TELEFONE", 10) => Some(format!(
            "({}) {}-{}",
            &digits[0..2],
            &digits[2..6],
            &digits[6..10]
        )),
        ("TELEFONE", 11) => Some(format!(
            "({}) {}-{}",
            &digits[0..2],
            &digits[2..7],
            &digits[7..11]
        )),
        ("TELEFONE", 12) if digits.starts_with("55") => Some(format!(
            "+55 ({}) {}-{}",
            &digits[2..4],
            &digits[4..8],
            &digits[8..12]
        )),
        ("TELEFONE", 13) if digits.starts_with("55") => Some(format!(
            "+55 ({}) {}-{}",
            &digits[2..4],
            &digits[4..9],
            &digits[9..13]
        )),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct QueridoDiarioResponse {
    #[serde(default)]
    gazettes: Vec<QueridoDiarioGazette>,
}

#[derive(Debug, Deserialize)]
struct QueridoDiarioGazette {
    date: Option<String>,
    url: Option<String>,
    txt_url: Option<String>,
    territory_name: Option<String>,
    state_code: Option<String>,
    edition: Option<String>,
    #[serde(default)]
    excerpts: Vec<String>,
}

fn normalize_querido_diario(
    data: QueridoDiarioResponse,
    max_results: usize,
) -> Vec<PublicSearchResult> {
    data.gazettes
        .into_iter()
        .filter_map(|gazette| {
            let url = gazette.url.or(gazette.txt_url)?.trim().to_owned();
            if url.is_empty() {
                return None;
            }
            let territory = gazette
                .territory_name
                .as_deref()
                .unwrap_or("Município não informado");
            let state = gazette.state_code.as_deref().unwrap_or("--");
            let edition = gazette
                .edition
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            let title = edition
                .map(|value| format!("Diário Oficial de {territory}/{state} — Edição {value}"))
                .unwrap_or_else(|| format!("Diário Oficial de {territory}/{state}"));
            let description =
                (!gazette.excerpts.is_empty()).then(|| strip_html(&gazette.excerpts.join(" … ")));
            let mut details = vec![format!("Município: {territory}/{state}")];
            if let Some(value) = edition {
                details.push(format!("Edição: {value}"));
            }
            Some(PublicSearchResult {
                title,
                url,
                description,
                published_at: gazette.date,
                source: format!("{territory}/{state}"),
                details: Some(details.join("\n")),
            })
        })
        .take(max_results)
        .collect()
}

#[derive(Debug, Deserialize)]
struct OpenAlexResponse {
    #[serde(default)]
    results: Vec<OpenAlexWork>,
}

#[derive(Debug, Deserialize)]
struct OpenAlexWork {
    id: Option<String>,
    doi: Option<String>,
    display_name: Option<String>,
    publication_date: Option<String>,
    publication_year: Option<i32>,
    cited_by_count: Option<u64>,
    #[serde(default)]
    authorships: Vec<OpenAlexAuthorship>,
    primary_location: Option<OpenAlexLocation>,
}

#[derive(Debug, Deserialize)]
struct OpenAlexAuthorship {
    author: Option<OpenAlexNamedEntity>,
    #[serde(default)]
    institutions: Vec<OpenAlexNamedEntity>,
}

#[derive(Debug, Deserialize)]
struct OpenAlexNamedEntity {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAlexLocation {
    landing_page_url: Option<String>,
    source: Option<OpenAlexNamedEntity>,
}

fn normalize_openalex(data: OpenAlexResponse, max_results: usize) -> Vec<PublicSearchResult> {
    data.results
        .into_iter()
        .filter_map(|work| {
            let doi_url = work.doi.as_deref().map(normalize_doi_url);
            let url = work
                .primary_location
                .as_ref()
                .and_then(|location| location.landing_page_url.clone())
                .or_else(|| doi_url.clone())
                .or(work.id.clone())?;
            let title = work.display_name.filter(|value| !value.trim().is_empty())?;
            let authors = unique_names(
                work.authorships
                    .iter()
                    .filter_map(|authorship| authorship.author.as_ref()?.display_name.clone()),
            );
            let institutions = unique_names(
                work.authorships
                    .iter()
                    .flat_map(|authorship| authorship.institutions.iter())
                    .filter_map(|institution| institution.display_name.clone()),
            );
            let journal = work
                .primary_location
                .as_ref()
                .and_then(|location| location.source.as_ref())
                .and_then(|source| source.display_name.clone());
            let mut details = Vec::new();
            if !authors.is_empty() {
                details.push(format!("Autores: {}", authors.join(", ")));
            }
            if !institutions.is_empty() {
                details.push(format!("Instituições: {}", institutions.join(", ")));
            }
            if let Some(doi) = doi_url {
                details.push(format!("DOI: {doi}"));
            }
            if let Some(citations) = work.cited_by_count {
                details.push(format!("Citações: {citations}"));
            }
            if work.publication_date.is_none()
                && let Some(year) = work.publication_year
            {
                details.push(format!("Ano: {year}"));
            }
            Some(PublicSearchResult {
                title,
                url,
                description: (!authors.is_empty()).then(|| authors.join(", ")),
                published_at: work.publication_date,
                source: journal.unwrap_or_else(|| "OpenAlex".to_owned()),
                details: (!details.is_empty()).then(|| details.join("\n")),
            })
        })
        .take(max_results)
        .collect()
}

fn normalize_doi_url(value: &str) -> String {
    if value.starts_with("http://") || value.starts_with("https://") {
        value.to_owned()
    } else {
        format!("https://doi.org/{}", value.trim_start_matches("doi:"))
    }
}

fn unique_names(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut names = Vec::new();
    for value in values {
        let value = value.trim().to_owned();
        if !value.is_empty() && !names.contains(&value) {
            names.push(value);
        }
    }
    names
}

#[derive(Debug, Clone, Default)]
struct InlabsDocument {
    title: String,
    summary: String,
    text: String,
    url: String,
    published_at: Option<String>,
    agency: Option<String>,
    section: Option<String>,
    edition: Option<String>,
    page: Option<String>,
}

fn parse_inlabs_zip(bytes: Vec<u8>) -> Result<Vec<InlabsDocument>, ProviderFailure> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
        tracing::warn!(provider = "INLABS", error = %error, "ZIP inválido");
        ProviderFailure::new("o INLABS retornou um arquivo ZIP inválido")
    })?;
    let mut documents = Vec::new();
    for index in 0..archive.len().min(INLABS_MAX_FILES_PER_ZIP) {
        let mut file = archive.by_index(index).map_err(|error| {
            tracing::warn!(provider = "INLABS", error = %error, "entrada ZIP inválida");
            ProviderFailure::new("não foi possível ler um arquivo do DOU")
        })?;
        if file.is_dir()
            || !file.name().to_ascii_lowercase().ends_with(".xml")
            || file.size() > INLABS_MAX_XML_BYTES
        {
            continue;
        }
        let mut xml = String::new();
        file.read_to_string(&mut xml).map_err(|error| {
            tracing::warn!(provider = "INLABS", error = %error, "XML do ZIP ilegível");
            ProviderFailure::new("não foi possível ler um XML do DOU")
        })?;
        if let Some(document) = parse_inlabs_xml(&xml) {
            documents.push(document);
        }
    }
    Ok(documents)
}

fn parse_inlabs_xml(xml: &str) -> Option<InlabsDocument> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut document = InlabsDocument::default();
    let mut current_tag = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                current_tag = String::from_utf8_lossy(element.name().as_ref()).to_string();
                if current_tag == "article" {
                    for attribute in element.attributes().flatten() {
                        let key = String::from_utf8_lossy(attribute.key.as_ref());
                        let value = attribute
                            .decode_and_unescape_value(reader.decoder())
                            .map(|value| value.into_owned())
                            .unwrap_or_default();
                        match key.as_ref() {
                            "name" | "artType" if document.title.is_empty() => {
                                document.title = value
                            }
                            "pubDate" => document.published_at = normalize_inlabs_date(&value),
                            "pdfPage" => document.url = value,
                            "artCategory" => document.agency = non_empty(value),
                            "pubName" => document.section = non_empty(value),
                            "editionNumber" => document.edition = non_empty(value),
                            "numberPage" => document.page = non_empty(value),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Text(text)) => {
                let value = text
                    .decode()
                    .map(|value| value.into_owned())
                    .unwrap_or_default();
                assign_inlabs_text(&mut document, &current_tag, value);
            }
            Ok(Event::CData(text)) => {
                let value = text
                    .decode()
                    .map(|value| value.into_owned())
                    .unwrap_or_default();
                assign_inlabs_text(&mut document, &current_tag, strip_html(&value));
            }
            Ok(Event::End(_)) => current_tag.clear(),
            Ok(Event::Eof) => break,
            Err(error) => {
                tracing::warn!(provider = "INLABS", error = %error, "XML inválido ignorado");
                return None;
            }
            _ => {}
        }
    }
    if document.title.is_empty() {
        document.title = "Publicação do Diário Oficial da União".to_owned();
    }
    if document.url.is_empty() {
        return None;
    }
    Some(document)
}

fn assign_inlabs_text(document: &mut InlabsDocument, tag: &str, value: String) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    match tag {
        "Identifica" => document.title = value.to_owned(),
        "Ementa" => document.summary = value.to_owned(),
        "Texto" => document.text = value.to_owned(),
        _ => {}
    }
}

fn normalize_inlabs_date(value: &str) -> Option<String> {
    ["%d/%m/%Y", "%Y-%m-%d"]
        .into_iter()
        .find_map(|format| NaiveDate::parse_from_str(value, format).ok())
        .map(|date| date.format("%Y-%m-%d").to_string())
}

fn search_inlabs_documents(
    documents: &[InlabsDocument],
    parameter: &ParametroBusca,
    max_results: usize,
) -> Vec<PublicSearchResult> {
    let query = parameter.valor.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    documents
        .iter()
        .filter_map(|document| {
            let searchable = format!("{} {} {}", document.title, document.summary, document.text);
            let position = searchable.to_lowercase().find(&query)?;
            let description = excerpt_around(&searchable, position, query.len(), 500);
            let mut details = Vec::new();
            if let Some(value) = document.agency.as_deref() {
                details.push(format!("Órgão: {value}"));
            }
            if let Some(value) = document.section.as_deref() {
                details.push(format!("Seção: {value}"));
            }
            if let Some(value) = document.edition.as_deref() {
                details.push(format!("Edição: {value}"));
            }
            if let Some(value) = document.page.as_deref() {
                details.push(format!("Página: {value}"));
            }
            Some(PublicSearchResult {
                title: document.title.clone(),
                url: document.url.clone(),
                description: Some(description),
                published_at: document.published_at.clone(),
                source: document
                    .agency
                    .clone()
                    .unwrap_or_else(|| "Diário Oficial da União".to_owned()),
                details: (!details.is_empty()).then(|| details.join("\n")),
            })
        })
        .take(max_results)
        .collect()
}

fn excerpt_around(
    text: &str,
    byte_position: usize,
    query_bytes: usize,
    max_chars: usize,
) -> String {
    let char_position = text[..byte_position.min(text.len())].chars().count();
    let query_chars = text
        [byte_position.min(text.len())..byte_position.saturating_add(query_bytes).min(text.len())]
        .chars()
        .count();
    let chars: Vec<char> = text.chars().collect();
    let half = max_chars / 2;
    let start = char_position.saturating_sub(half);
    let end = (char_position + query_chars + half).min(chars.len());
    let mut excerpt: String = chars[start..end].iter().collect();
    excerpt = excerpt.split_whitespace().collect::<Vec<_>>().join(" ");
    if start > 0 {
        excerpt.insert_str(0, "… ");
    }
    if end < chars.len() {
        excerpt.push_str(" …");
    }
    excerpt
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn strip_html(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut inside_tag = false;
    for character in value.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                result.push(' ');
            }
            _ if !inside_tag => result.push(character),
            _ => {}
        }
    }
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter(provider: &str) -> ParametroBusca {
        ParametroBusca {
            id: 1,
            pessoa_id: 1,
            tipo: "NOME".to_owned(),
            valor: "Maria Silva".to_owned(),
            provider: provider.to_owned(),
            ativo: true,
        }
    }

    #[test]
    fn accepts_all_providers_and_rejects_invalid_value() {
        assert_eq!(SearchProvider::parse(""), Ok(SearchProvider::Searxng));
        assert_eq!(
            SearchProvider::parse("SEARXNG"),
            Ok(SearchProvider::Searxng)
        );
        assert_eq!(
            SearchProvider::parse("QUERIDO_DIARIO"),
            Ok(SearchProvider::QueridoDiario)
        );
        assert_eq!(SearchProvider::parse("INLABS"), Ok(SearchProvider::Inlabs));
        assert_eq!(
            SearchProvider::parse("OPENALEX"),
            Ok(SearchProvider::OpenAlex)
        );
        assert!(SearchProvider::parse("https://example.com").is_err());
    }

    #[test]
    fn preserves_searxng_query_variants() {
        assert_eq!(
            queries_for_searxng(&parameter("SEARXNG")),
            vec!["\"Maria Silva\"", "Maria Silva"]
        );
    }

    #[test]
    fn normalizes_querido_diario_result_and_empty_response() {
        let data: QueridoDiarioResponse = serde_json::from_str(
            r#"{"gazettes":[{"date":"2026-08-10","url":"https://example.org/doc.pdf","territory_name":"Niterói","state_code":"RJ","edition":"12","excerpts":["<em>convocação</em> pública"]}]}"#,
        ).unwrap();
        let results = normalize_querido_diario(data, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].published_at.as_deref(), Some("2026-08-10"));
        assert_eq!(results[0].source, "Niterói/RJ");
        assert_eq!(
            results[0].description.as_deref(),
            Some("convocação pública")
        );
        assert!(
            normalize_querido_diario(QueridoDiarioResponse { gazettes: vec![] }, 10).is_empty()
        );
    }

    #[test]
    fn normalizes_openalex_metadata_and_doi() {
        let data: OpenAlexResponse = serde_json::from_str(
            r#"{"results":[{"id":"https://openalex.org/W1","doi":"https://doi.org/10.1/demo","display_name":"Artigo","publication_date":"2026-08-10","cited_by_count":7,"authorships":[{"author":{"display_name":"Ana"},"institutions":[{"display_name":"Universidade"}]}],"primary_location":{"landing_page_url":"https://journal.example/article","source":{"display_name":"Revista"}}}]}"#,
        ).unwrap();
        let results = normalize_openalex(data, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "Revista");
        let details = results[0].details.as_deref().unwrap();
        assert!(details.contains("DOI: https://doi.org/10.1/demo"));
        assert!(details.contains("Citações: 7"));
        assert!(details.contains("Instituições: Universidade"));
    }

    #[test]
    fn parses_and_searches_inlabs_xml() {
        let xml = r#"<article name="Portaria" pubDate="10/08/2026" pdfPage="https://in.gov.br/doc.pdf" artCategory="Ministério da Educação" pubName="DO1" editionNumber="150" numberPage="4"><Identifica>EDITAL Nº 12</Identifica><Ementa>Convocação de Maria Silva</Ementa><Texto><![CDATA[<p>Texto complementar</p>]]></Texto></article>"#;
        let document = parse_inlabs_xml(xml).unwrap();
        assert_eq!(document.published_at.as_deref(), Some("2026-08-10"));
        let results = search_inlabs_documents(&[document], &parameter("INLABS"), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "EDITAL Nº 12");
        assert!(
            results[0]
                .details
                .as_deref()
                .unwrap()
                .contains("Seção: DO1")
        );
    }

    #[test]
    fn classifies_http_and_timeout_errors() {
        assert!(ensure_success("OpenAlex", StatusCode::OK).is_ok());
        assert!(
            ensure_success("OpenAlex", StatusCode::TOO_MANY_REQUESTS)
                .unwrap_err()
                .user_message
                .contains("limite")
        );
        assert!(
            ensure_success("OpenAlex", StatusCode::INTERNAL_SERVER_ERROR)
                .unwrap_err()
                .user_message
                .contains("indisponível")
        );
    }

    #[tokio::test]
    async fn maps_real_request_timeout_without_exposing_details() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        let client = Client::builder()
            .timeout(Duration::from_millis(20))
            .build()
            .unwrap();
        let error = client
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap_err();
        server.abort();
        assert!(error.is_timeout());
        assert_eq!(
            request_error_message(&error),
            "tempo limite da consulta excedido"
        );
    }

    #[test]
    fn completes_searxng_endpoint() {
        assert_eq!(
            endpoint_searxng("https://busca.exemplo.org/instancia")
                .unwrap()
                .as_str(),
            "https://busca.exemplo.org/instancia/search"
        );
    }

    #[test]
    fn explains_searxng_403() {
        let warning = searxng_status_message("NOME", StatusCode::FORBIDDEN);
        assert!(warning.contains("API JSON"));
        assert!(warning.contains("search.formats"));
    }
}
