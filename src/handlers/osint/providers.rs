use std::{
    collections::{BTreeSet, HashSet},
    io::{Cursor, Read},
    time::Duration,
};

use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use quick_xml::{Reader, events::Event};
use reqwest::{Client, StatusCode, Url, header, redirect::Policy};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::{config::Config, models::ParametroBusca};

const USER_AGENT: &str = "AgendarX-OSINT/0.4 (+pesquisa-publica)";
const QUERIDO_DIARIO_URL: &str = "https://api.queridodiario.ok.org.br/gazettes";
const OPENALEX_URL: &str = "https://api.openalex.org/works";
const PNCP_DEFAULT_BASE_URL: &str = "https://pncp.gov.br";
const CGU_DEFAULT_BASE_URL: &str = "https://api.portaldatransparencia.gov.br/api-de-dados";
const TSE_DEFAULT_BASE_URL: &str = "https://divulgacandcontas.tse.jus.br/divulga/rest/v1";
const INLABS_LOGIN_URL: &str = "https://inlabs.in.gov.br/logar.php";
const INLABS_DOWNLOAD_URL: &str = "https://inlabs.in.gov.br/index.php";
const INLABS_SECTIONS: [&str; 6] = ["DO1", "DO2", "DO3", "DO1E", "DO2E", "DO3E"];
const INLABS_MAX_ZIP_BYTES: usize = 100 * 1024 * 1024;
const INLABS_MAX_XML_BYTES: u64 = 4 * 1024 * 1024;
const INLABS_MAX_FILES_PER_ZIP: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SearchProvider {
    Searxng,
    QueridoDiario,
    Inlabs,
    OpenAlex,
    Datajud,
    Pncp,
    Cgu,
    Tse,
}

impl SearchProvider {
    pub const ALL: [SearchProvider; 8] = [
        Self::Pncp,
        Self::Cgu,
        Self::Tse,
        Self::QueridoDiario,
        Self::Datajud,
        Self::OpenAlex,
        Self::Searxng,
        Self::Inlabs,
    ];

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_uppercase().as_str() {
            "" | "SEARXNG" => Ok(Self::Searxng),
            "QUERIDO_DIARIO" => Ok(Self::QueridoDiario),
            "INLABS" => Ok(Self::Inlabs),
            "OPENALEX" => Ok(Self::OpenAlex),
            "DATAJUD" => Ok(Self::Datajud),
            "PNCP" => Ok(Self::Pncp),
            "CGU" => Ok(Self::Cgu),
            "TSE" => Ok(Self::Tse),
            _ => Err(
                "fonte inválida; use COMPATIVEIS, SEARXNG, QUERIDO_DIARIO, INLABS, OPENALEX, DATAJUD, PNCP, CGU ou TSE"
                    .to_owned(),
            ),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Searxng => "SEARXNG",
            Self::QueridoDiario => "QUERIDO_DIARIO",
            Self::Inlabs => "INLABS",
            Self::OpenAlex => "OPENALEX",
            Self::Datajud => "DATAJUD",
            Self::Pncp => "PNCP",
            Self::Cgu => "CGU",
            Self::Tse => "TSE",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Searxng => "SearXNG",
            Self::QueridoDiario => "Querido Diário",
            Self::Inlabs => "INLABS / DOU",
            Self::OpenAlex => "OpenAlex",
            Self::Datajud => "DataJud / CNJ",
            Self::Pncp => "PNCP (Contratações Públicas)",
            Self::Cgu => "CGU (Portal da Transparência)",
            Self::Tse => "DivulgaCandContas / TSE",
        }
    }

    pub fn suporta_tipo(self, tipo: &str) -> bool {
        match self {
            Self::Pncp => matches!(tipo, "CNPJ" | "NOME" | "TERMO" | "PROCESSO"),
            Self::Cgu => matches!(tipo, "CPF" | "CNPJ" | "NOME"),
            Self::Tse => matches!(tipo, "NOME" | "CPF" | "CNPJ"),
            Self::Datajud => tipo == "PROCESSO",
            Self::OpenAlex => matches!(tipo, "NOME" | "TERMO"),
            Self::QueridoDiario | Self::Searxng | Self::Inlabs => true,
        }
    }

    pub fn suporta_parametro(self, tipo: &str, valor: &str) -> bool {
        if !self.suporta_tipo(tipo) {
            return false;
        }
        if self == Self::Datajud {
            return datajud_query(valor).is_ok();
        }
        true
    }
}

pub fn fontes_padrao_ativas() -> Vec<String> {
    SearchProvider::ALL
        .iter()
        .map(|provider| provider.as_str().to_owned())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OsintConfiguracao {
    pub searxng_url: Option<String>,
    pub cgu_api_key: Option<String>,
    pub datajud_api_key: Option<String>,
    pub openalex_api_key: Option<String>,
    pub inlabs_username: Option<String>,
    pub inlabs_password: Option<String>,
    pub inlabs_lookback_days: u64,
    pub pncp_base_url: String,
    pub pncp_tipos_documento: String,
    pub pncp_retroativo_dias: u64,
    pub cgu_base_url: String,
    pub tse_base_url: String,
    pub tse_anos: String,
    pub tse_abrangencias: String,
    pub tse_cargos: String,
    pub osint_timeout_seconds: u64,
    pub osint_max_results: usize,
    pub fontes_ativas: Vec<String>,
}

impl Default for OsintConfiguracao {
    fn default() -> Self {
        Self {
            searxng_url: None,
            cgu_api_key: None,
            datajud_api_key: None,
            openalex_api_key: None,
            inlabs_username: None,
            inlabs_password: None,
            inlabs_lookback_days: 1,
            pncp_base_url: PNCP_DEFAULT_BASE_URL.to_owned(),
            pncp_tipos_documento: "edital|ata|contrato".to_owned(),
            pncp_retroativo_dias: 365,
            cgu_base_url: CGU_DEFAULT_BASE_URL.to_owned(),
            tse_base_url: TSE_DEFAULT_BASE_URL.to_owned(),
            tse_anos: "2024,2022".to_owned(),
            tse_abrangencias: "BR,SP,RJ,MG,DF".to_owned(),
            tse_cargos: "1,3,5,6,7,11,13".to_owned(),
            osint_timeout_seconds: 20,
            osint_max_results: 15,
            fontes_ativas: fontes_padrao_ativas(),
        }
    }
}

impl OsintConfiguracao {
    pub fn from_config(config: &Config) -> Self {
        Self {
            searxng_url: config.searxng_url.clone(),
            cgu_api_key: config.cgu_api_key.clone(),
            datajud_api_key: config.datajud_api_key.clone(),
            openalex_api_key: config.openalex_api_key.clone(),
            inlabs_username: config.inlabs_username.clone(),
            inlabs_password: config.inlabs_password.clone(),
            inlabs_lookback_days: config.inlabs_lookback_days,
            osint_timeout_seconds: config.osint_timeout_seconds,
            osint_max_results: config.osint_max_results,
            ..Self::default()
        }
    }

    pub fn aplicar_json_db(&mut self, json_str: &str) {
        let Ok(valor) = serde_json::from_str::<serde_json::Value>(json_str) else {
            return;
        };
        let Some(obj) = valor.as_object() else {
            return;
        };
        if let Some(v) = obj.get("searxng_url") {
            self.searxng_url = v
                .as_str()
                .map(|s| s.trim().trim_end_matches('/').to_owned())
                .filter(|s| !s.is_empty());
        }
        if let Some(v) = obj.get("cgu_api_key") {
            self.cgu_api_key = v
                .as_str()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty());
        }
        if let Some(v) = obj.get("datajud_api_key") {
            self.datajud_api_key = v
                .as_str()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty());
        }
        if let Some(v) = obj.get("openalex_api_key") {
            self.openalex_api_key = v
                .as_str()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty());
        }
        if let Some(v) = obj.get("inlabs_username") {
            self.inlabs_username = v
                .as_str()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty());
        }
        if let Some(v) = obj.get("inlabs_password") {
            self.inlabs_password = v
                .as_str()
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty());
        }
        if let Some(v) = obj.get("inlabs_lookback_days").and_then(|v| v.as_u64())
            && (1..=7).contains(&v)
        {
            self.inlabs_lookback_days = v;
        }
        if let Some(v) = obj.get("pncp_base_url").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.pncp_base_url = v.trim().trim_end_matches('/').to_owned();
        }
        if let Some(v) = obj.get("pncp_tipos_documento").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.pncp_tipos_documento = v.trim().to_owned();
        }
        if let Some(v) = obj.get("pncp_retroativo_dias").and_then(|v| v.as_u64())
            && (1..=3650).contains(&v)
        {
            self.pncp_retroativo_dias = v;
        }
        if let Some(v) = obj.get("cgu_base_url").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.cgu_base_url = v.trim().trim_end_matches('/').to_owned();
        }
        if let Some(v) = obj.get("tse_base_url").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.tse_base_url = v.trim().trim_end_matches('/').to_owned();
        }
        if let Some(v) = obj.get("tse_anos").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.tse_anos = v.trim().to_owned();
        }
        if let Some(v) = obj.get("tse_abrangencias").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.tse_abrangencias = v.trim().to_owned();
        }
        if let Some(v) = obj.get("tse_cargos").and_then(|v| v.as_str())
            && !v.trim().is_empty()
        {
            self.tse_cargos = v.trim().to_owned();
        }
        if let Some(v) = obj.get("osint_timeout_seconds").and_then(|v| v.as_u64())
            && (5..=120).contains(&v)
        {
            self.osint_timeout_seconds = v;
        }
        if let Some(v) = obj.get("osint_max_results").and_then(|v| v.as_u64())
            && (1..=100).contains(&v)
        {
            self.osint_max_results = v as usize;
        }
        if let Some(arr) = obj.get("fontes_ativas").and_then(|v| v.as_array()) {
            let mut validas = Vec::new();
            for item in arr.iter().filter_map(|v| v.as_str()) {
                if let Ok(provider) = SearchProvider::parse(item) {
                    let code = provider.as_str().to_owned();
                    if !validas.contains(&code) {
                        validas.push(code);
                    }
                }
            }
            if !validas.is_empty() {
                self.fontes_ativas = validas;
            }
        }
    }

    pub fn provider_configurado(&self, provider: SearchProvider) -> bool {
        match provider {
            SearchProvider::Pncp
            | SearchProvider::Tse
            | SearchProvider::QueridoDiario
            | SearchProvider::OpenAlex
            | SearchProvider::Datajud => true,
            SearchProvider::Cgu => self
                .cgu_api_key
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty()),
            SearchProvider::Searxng => self
                .searxng_url
                .as_deref()
                .is_some_and(|v| endpoint_searxng(v).is_ok()),
            SearchProvider::Inlabs => {
                self.inlabs_username
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty())
                    && self
                        .inlabs_password
                        .as_deref()
                        .is_some_and(|v| !v.trim().is_empty())
            }
        }
    }

    pub fn provider_ativo(&self, provider: SearchProvider) -> bool {
        self.fontes_ativas
            .iter()
            .any(|f| f.eq_ignore_ascii_case(provider.as_str()))
    }

    pub fn providers_compativeis_para(&self, tipo: &str, valor: &str) -> Vec<SearchProvider> {
        SearchProvider::ALL
            .into_iter()
            .filter(|provider| {
                provider.suporta_parametro(tipo, valor)
                    && self.provider_ativo(*provider)
                    && self.provider_configurado(*provider)
            })
            .collect()
    }

    pub fn expandir_parametros(
        &self,
        parametros: Vec<ParametroBusca>,
        expandir_todos_compativeis: bool,
    ) -> Vec<ParametroBusca> {
        let mut expandidos = Vec::new();
        let mut vistos = HashSet::new();

        for parametro in parametros {
            let modo_compativeis = parametro.provider.eq_ignore_ascii_case("COMPATIVEIS")
                || expandir_todos_compativeis;
            if modo_compativeis {
                let mut alvos = Vec::new();
                if !parametro.provider.eq_ignore_ascii_case("COMPATIVEIS")
                    && let Ok(explicito) = SearchProvider::parse(&parametro.provider)
                {
                    alvos.push(explicito);
                }
                for compativel in self.providers_compativeis_para(&parametro.tipo, &parametro.valor)
                {
                    if !alvos.contains(&compativel) {
                        alvos.push(compativel);
                    }
                }
                if alvos.is_empty() {
                    alvos.push(SearchProvider::QueridoDiario);
                }
                for provider in alvos {
                    let chave = (
                        parametro.tipo.to_ascii_uppercase(),
                        parametro.valor.trim().to_lowercase(),
                        provider.as_str().to_owned(),
                    );
                    if vistos.insert(chave) {
                        let mut clone = parametro.clone();
                        clone.provider = provider.as_str().to_owned();
                        expandidos.push(clone);
                    }
                }
            } else {
                let chave = (
                    parametro.tipo.to_ascii_uppercase(),
                    parametro.valor.trim().to_lowercase(),
                    parametro.provider.to_ascii_uppercase(),
                );
                if vistos.insert(chave) {
                    expandidos.push(parametro);
                }
            }
        }
        expandidos
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
    cgu_api_key: Option<String>,
    cgu_base_url: String,
    pncp_base_url: String,
    pncp_tipos_documento: String,
    pncp_retroativo_dias: u64,
    tse_base_url: String,
    tse_anos: String,
    tse_abrangencias: String,
    tse_cargos: String,
    openalex_api_key: Option<String>,
    datajud_api_key: Option<String>,
    inlabs_username: Option<String>,
    inlabs_password: Option<String>,
    inlabs_lookback_days: u64,
    max_results: usize,
    inlabs_cache: InlabsCache,
}

impl PublicSearchProviders {
    #[allow(dead_code)]
    pub fn new(config: &Config) -> Result<Self, String> {
        Self::from_osint_config(&OsintConfiguracao::from_config(config))
    }

    pub fn from_osint_config(config: &OsintConfiguracao) -> Result<Self, String> {
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
                Some("SearXNG não configurado. Configure a URL do SearXNG em Configurações > Pesquisa Pública (OSINT).".to_owned()),
            ),
        };

        Ok(Self {
            client,
            searxng_endpoint,
            searxng_configuration_error,
            cgu_api_key: config.cgu_api_key.clone(),
            cgu_base_url: config.cgu_base_url.clone(),
            pncp_base_url: config.pncp_base_url.clone(),
            pncp_tipos_documento: config.pncp_tipos_documento.clone(),
            pncp_retroativo_dias: config.pncp_retroativo_dias,
            tse_base_url: config.tse_base_url.clone(),
            tse_anos: config.tse_anos.clone(),
            tse_abrangencias: config.tse_abrangencias.clone(),
            tse_cargos: config.tse_cargos.clone(),
            openalex_api_key: config.openalex_api_key.clone(),
            datajud_api_key: config.datajud_api_key.clone(),
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
            SearchProvider::Datajud => self.search_datajud(parameter).await,
            SearchProvider::Pncp => self.search_pncp(parameter).await,
            SearchProvider::Cgu => self.search_cgu(parameter).await,
            SearchProvider::Tse => self.search_tse(parameter).await,
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

    async fn search_datajud(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        let (number, tribunal) = datajud_query(&parameter.valor).map_err(ProviderFailure::new)?;
        // Public key published by CNJ; deployment override supports key rotation.
        let key = self
            .datajud_api_key
            .as_deref()
            .unwrap_or("cDZHYzlZa0JadVREZDJCendQbXY6SkJlTzNjLV9TRENyQk1RdnFKZGRQdw==");
        let response = self.client
            .post(format!("https://api-publica.datajud.cnj.jus.br/api_publica_{tribunal}/_search"))
            .header(header::AUTHORIZATION, format!("APIKey {key}"))
            .json(&serde_json::json!({"size": self.max_results, "query": {"match": {"numeroProcesso": number}}}))
            .send().await.map_err(|error| provider_request_error("DataJud", error))?;
        ensure_success("DataJud", response.status())?;
        let data: DatajudResponse = response
            .json()
            .await
            .map_err(|_| ProviderFailure::new("a API retornou uma resposta inválida"))?;
        Ok(SearchExecution {
            results: normalize_datajud(data, self.max_results),
            requests_executed: 1,
            ..SearchExecution::default()
        })
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

    async fn search_pncp(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        if !SearchProvider::Pncp.suporta_tipo(&parameter.tipo) {
            return Err(ProviderFailure::new(
                "PNCP aceita parâmetros dos tipos CNPJ, NOME, TERMO ou PROCESSO",
            ));
        }
        let base = self.pncp_base_url.trim_end_matches('/');
        let query = if parameter.tipo == "CNPJ" {
            let digits: String = parameter
                .valor
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            if digits.len() == 14 {
                digits
            } else {
                parameter.valor.trim().to_owned()
            }
        } else {
            parameter.valor.trim().to_owned()
        };

        let mut execution = SearchExecution::default();
        let mut urls = HashSet::new();
        let mut any_success = false;

        let search_url = format!("{base}/api/search/");
        execution.requests_executed += 1;
        match self
            .client
            .get(&search_url)
            .header(header::ACCEPT, "application/json")
            .query(&[
                ("q", query.as_str()),
                ("tipos_documento", self.pncp_tipos_documento.as_str()),
                ("ordenacao", "-data"),
                ("pagina", "1"),
                ("tam_pagina", &self.max_results.to_string()),
            ])
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                if let Ok(data) = response.json::<PncpSearchResponse>().await {
                    any_success = true;
                    for item in normalize_pncp_search(data, base, self.max_results) {
                        if urls.insert(item.url.clone()) {
                            execution.results.push(item);
                        }
                    }
                }
            }
            Ok(response) if response.status() == StatusCode::NO_CONTENT => {
                any_success = true;
            }
            Ok(response) => {
                if parameter.tipo != "CNPJ" {
                    ensure_success("PNCP", response.status())?;
                }
            }
            Err(error) => {
                if parameter.tipo != "CNPJ" {
                    return Err(provider_request_error("PNCP", error));
                }
            }
        }

        if parameter.tipo == "CNPJ" && execution.results.len() < self.max_results {
            let digits: String = parameter
                .valor
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            if digits.len() == 14 {
                let hoje = Local::now().date_naive();
                let inicio = hoje - ChronoDuration::days(self.pncp_retroativo_dias as i64);
                let data_ini = inicio.format("%Y%m%d").to_string();
                let data_fim = hoje.format("%Y%m%d").to_string();
                let tam = self.max_results.max(10).to_string();
                let contratos_url = format!("{base}/api/consulta/v1/contratos");
                execution.requests_executed += 1;
                if let Ok(response) = self
                    .client
                    .get(&contratos_url)
                    .header(header::ACCEPT, "application/json")
                    .query(&[
                        ("dataInicial", data_ini.as_str()),
                        ("dataFinal", data_fim.as_str()),
                        ("niFornecedor", digits.as_str()),
                        ("pagina", "1"),
                        ("tamanhoPagina", tam.as_str()),
                    ])
                    .send()
                    .await
                {
                    if response.status().is_success() {
                        if let Ok(data) = response.json::<PncpContratosResponse>().await {
                            any_success = true;
                            for item in normalize_pncp_contratos(data, base, self.max_results) {
                                if execution.results.len() >= self.max_results {
                                    break;
                                }
                                if urls.insert(item.url.clone()) {
                                    execution.results.push(item);
                                }
                            }
                        }
                    } else if response.status() == StatusCode::NO_CONTENT {
                        any_success = true;
                    }
                }
            }
        }

        if !any_success {
            return Err(ProviderFailure::new(
                "não foi possível obter resposta válida do PNCP",
            ));
        }

        Ok(execution)
    }

    async fn search_cgu(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        if !SearchProvider::Cgu.suporta_tipo(&parameter.tipo) {
            return Err(ProviderFailure::new(
                "CGU (Portal da Transparência) aceita parâmetros dos tipos CPF, CNPJ ou NOME",
            ));
        }
        let api_key = self
            .cgu_api_key
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                ProviderFailure::new(
                    "Portal da Transparência (CGU) não configurado. Informe a Chave da API de Dados (chave-api-dados) em Configurações > Pesquisa Pública (OSINT).",
                )
            })?;

        let base = self.cgu_base_url.trim_end_matches('/');
        type CguEndpoint = (&'static str, String, Vec<(&'static str, String)>);
        let mut endpoints: Vec<CguEndpoint> = Vec::new();
        match parameter.tipo.as_str() {
            "CPF" => {
                let digits: String = parameter
                    .valor
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect();
                endpoints.push((
                    "CEIS",
                    format!("{base}/ceis"),
                    vec![("codigoSancionado", digits.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "CNEP",
                    format!("{base}/cnep"),
                    vec![("codigoSancionado", digits.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "SERVIDORES",
                    format!("{base}/servidores"),
                    vec![("cpf", digits), ("pagina", "1".into())],
                ));
            }
            "CNPJ" => {
                let digits: String = parameter
                    .valor
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect();
                endpoints.push((
                    "CEIS",
                    format!("{base}/ceis"),
                    vec![("codigoSancionado", digits.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "CNEP",
                    format!("{base}/cnep"),
                    vec![("codigoSancionado", digits.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "CEPIM",
                    format!("{base}/cepim"),
                    vec![("cnpjSancionado", digits.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "CONTRATOS",
                    format!("{base}/contratos/cpf-cnpj"),
                    vec![("cpfCnpj", digits), ("pagina", "1".into())],
                ));
            }
            "NOME" => {
                let nome = parameter.valor.trim().to_owned();
                endpoints.push((
                    "CEIS",
                    format!("{base}/ceis"),
                    vec![("nomeSancionado", nome.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "CNEP",
                    format!("{base}/cnep"),
                    vec![("nomeSancionado", nome.clone()), ("pagina", "1".into())],
                ));
                endpoints.push((
                    "SERVIDORES",
                    format!("{base}/servidores"),
                    vec![("nome", nome), ("pagina", "1".into())],
                ));
            }
            _ => {}
        }

        let mut execution = SearchExecution::default();
        let mut urls = HashSet::new();
        let mut successes = 0usize;
        let mut last_error: Option<ProviderFailure> = None;

        for (categoria, url, query) in endpoints {
            if execution.results.len() >= self.max_results {
                break;
            }
            execution.requests_executed += 1;
            let response = match self
                .client
                .get(&url)
                .header(header::ACCEPT, "application/json")
                .header("chave-api-dados", api_key)
                .query(&query)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    last_error = Some(provider_request_error("CGU", err));
                    continue;
                }
            };

            if response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::FORBIDDEN
            {
                return Err(ProviderFailure::new(
                    "acesso recusado pela API da CGU; verifique a chave-api-dados em Configurações",
                ));
            }
            if !response.status().is_success() {
                last_error = ensure_success("CGU", response.status()).err();
                continue;
            }
            let Ok(payload) = response.json::<serde_json::Value>().await else {
                last_error = Some(ProviderFailure::new("a API da CGU retornou JSON inválido"));
                continue;
            };
            successes += 1;
            for item in normalize_cgu_payload(categoria, &payload, self.max_results) {
                if execution.results.len() >= self.max_results {
                    break;
                }
                if urls.insert(item.url.clone()) {
                    execution.results.push(item);
                }
            }
        }

        if successes == 0
            && let Some(err) = last_error
        {
            return Err(err);
        }
        Ok(execution)
    }

    async fn search_tse(
        &self,
        parameter: &ParametroBusca,
    ) -> Result<SearchExecution, ProviderFailure> {
        if !SearchProvider::Tse.suporta_tipo(&parameter.tipo) {
            return Err(ProviderFailure::new(
                "DivulgaCandContas (TSE) aceita parâmetros dos tipos NOME, CPF ou CNPJ",
            ));
        }

        let base = self.tse_base_url.trim_end_matches('/');
        let anos: Vec<i32> = self
            .tse_anos
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect();
        let abrangencias: Vec<String> = self
            .tse_abrangencias
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        let cargos: Vec<String> = self
            .tse_cargos
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();

        let mut execution = SearchExecution::default();
        let mut urls = HashSet::new();
        let mut successes = 0usize;
        let mut last_error: Option<ProviderFailure> = None;

        for ano in if anos.is_empty() {
            vec![2024, 2022]
        } else {
            anos
        } {
            let eleicao_id = tse_eleicao_id_por_ano(ano);
            let eleicao_municipal = ano % 4 == 0;
            for ue in &abrangencias {
                if execution.results.len() >= self.max_results {
                    break;
                }
                let ue_numerico = ue.chars().all(|c| c.is_ascii_digit());
                if eleicao_municipal && !ue_numerico && ue.len() == 2 {
                    // Eleições municipais usam código TSE de município (5 dígitos); tentar busca direta ou pular UFs de 2 letras se houver códigos numéricos
                }
                if !eleicao_municipal && ue_numerico {
                    continue;
                }

                // 1) Tenta endpoint de busca de candidatos da circunscrição
                let search_url =
                    format!("{base}/candidatura/buscar/{ano}/{ue}/{eleicao_id}/candidatos");
                execution.requests_executed += 1;
                if let Ok(resp) = self
                    .client
                    .get(&search_url)
                    .header(header::ACCEPT, "application/json")
                    .query(&[("nome", parameter.valor.trim())])
                    .send()
                    .await
                    && resp.status().is_success()
                    && let Ok(payload) = resp.json::<serde_json::Value>().await
                {
                    successes += 1;
                    for item in normalize_tse_candidatos(
                        &payload,
                        ano,
                        eleicao_id,
                        ue,
                        parameter,
                        self.max_results,
                    ) {
                        if execution.results.len() >= self.max_results {
                            break;
                        }
                        if urls.insert(item.url.clone()) {
                            execution.results.push(item);
                        }
                    }
                    continue;
                }

                // 2) Consulta listagem por cargos configurados na circunscrição
                let cargos_ativos: Vec<&str> = if eleicao_municipal {
                    cargos
                        .iter()
                        .map(String::as_str)
                        .filter(|c| matches!(*c, "11" | "13"))
                        .collect()
                } else if ue == "BR" {
                    cargos
                        .iter()
                        .map(String::as_str)
                        .filter(|c| *c == "1")
                        .collect()
                } else {
                    cargos
                        .iter()
                        .map(String::as_str)
                        .filter(|c| matches!(*c, "3" | "5" | "6" | "7" | "8"))
                        .collect()
                };

                for cargo in cargos_ativos {
                    if execution.results.len() >= self.max_results {
                        break;
                    }
                    let list_url = format!(
                        "{base}/candidatura/listar/{ano}/{ue}/{eleicao_id}/{cargo}/candidatos"
                    );
                    execution.requests_executed += 1;
                    match self
                        .client
                        .get(&list_url)
                        .header(header::ACCEPT, "application/json")
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(payload) = resp.json::<serde_json::Value>().await {
                                successes += 1;
                                for item in normalize_tse_candidatos(
                                    &payload,
                                    ano,
                                    eleicao_id,
                                    ue,
                                    parameter,
                                    self.max_results,
                                ) {
                                    if execution.results.len() >= self.max_results {
                                        break;
                                    }
                                    if urls.insert(item.url.clone()) {
                                        execution.results.push(item);
                                    }
                                }
                            }
                        }
                        Ok(resp) if resp.status() == StatusCode::NOT_FOUND => {
                            successes += 1;
                        }
                        Ok(resp) => {
                            last_error =
                                ensure_success("DivulgaCandContas / TSE", resp.status()).err();
                        }
                        Err(err) => {
                            last_error =
                                Some(provider_request_error("DivulgaCandContas / TSE", err));
                        }
                    }
                }
            }
        }

        if successes == 0
            && let Some(err) = last_error
        {
            return Err(err);
        }
        Ok(execution)
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
        let is_xml = file
            .name()
            .ok()
            .is_some_and(|name| name.to_ascii_lowercase().ends_with(".xml"));
        if file.is_dir() || !is_xml || file.size() > INLABS_MAX_XML_BYTES {
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
                current_tag = element.name().as_ref().to_string();
                if current_tag == "article" {
                    for attribute in element.attributes().flatten() {
                        let key = attribute.key.as_ref();
                        let value = attribute
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map(|value| value.into_owned())
                            .unwrap_or_default();
                        match key {
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
                let value = quick_xml::escape::unescape(&text)
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| text.to_string());
                assign_inlabs_text(&mut document, &current_tag, value);
            }
            Ok(Event::CData(text)) => {
                let value = text.to_string();
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

// CNJ numbering: NNNNNNN-DD.AAAA.J.TR.OOOO. Only public tribunal aliases are accepted.
pub(super) fn datajud_query(value: &str) -> Result<(String, String), String> {
    let number: String = value.chars().filter(char::is_ascii_digit).collect();
    if number.len() != 20
        || value
            .chars()
            .any(|c| !c.is_ascii_digit() && c != '.' && c != '-')
    {
        return Err(
            "Informe o número CNJ do processo, com 20 dígitos (com ou sem pontuação)".into(),
        );
    }
    let region: usize = number[14..16].parse().unwrap_or(0);
    let states = [
        "ac", "al", "ap", "am", "ba", "ce", "dft", "es", "go", "ma", "mt", "ms", "mg", "pa", "pb",
        "pr", "pe", "pi", "rj", "rn", "rs", "ro", "rr", "sc", "se", "sp", "to",
    ];
    let tribunal = match (&number[13..14], region) {
        ("3", 0) => "stj".to_owned(),
        ("4", 1..=6) => format!("trf{region}"),
        ("5", 0) => "tst".to_owned(),
        ("5", 1..=24) => format!("trt{region}"),
        ("6", 0) => "tse".to_owned(),
        ("6", 1..=27) => format!("tre-{}", states[region - 1]),
        ("7", 0..=12) => "stm".to_owned(),
        ("8", 1..=27) => format!("tj{}", states[region - 1]),
        ("9", 13 | 21 | 26) => format!("tjm{}", states[region - 1]),
        _ => {
            return Err(
                "Tribunal do número informado não disponível na API Pública do DataJud".into(),
            );
        }
    };
    Ok((number, tribunal))
}

#[derive(Deserialize)]
struct DatajudResponse {
    hits: DatajudHits,
}
#[derive(Deserialize)]
struct DatajudHits {
    hits: Vec<DatajudHit>,
}
#[derive(Deserialize)]
struct DatajudHit {
    #[serde(rename = "_source")]
    source: serde_json::Value,
}

fn normalize_datajud(data: DatajudResponse, max: usize) -> Vec<PublicSearchResult> {
    data.hits.hits.into_iter().filter_map(|hit| {
        let value = hit.source;
        let number = value.get("numeroProcesso")?.as_str()?;
        let tribunal = value.get("tribunal").and_then(|v| v.as_str()).unwrap_or("CNJ");
        let class = value.pointer("/classe/nome").and_then(|v| v.as_str()).unwrap_or("Processo judicial");
        let court = value.pointer("/orgaoJulgador/nome").and_then(|v| v.as_str()).unwrap_or("");
        let grade = value.get("grau").and_then(|v| v.as_str()).unwrap_or("");
        let mut details = Vec::new();
        if let Some(date) = value.get("dataAjuizamento").and_then(|v| v.as_str()) {
            details.push(format!("Ajuizamento: {date}"));
        }
        if let Some(subjects) = value.get("assuntos").and_then(|v| v.as_array()) {
            let names = subjects.iter().filter_map(|v| v.get("nome").and_then(|v| v.as_str())).collect::<Vec<_>>();
            if !names.is_empty() { details.push(format!("Assuntos: {}", names.join(", "))); }
        }
        if let Some(movements) = value.get("movimentos").and_then(|v| v.as_array()) {
            let mut movements = movements.iter().collect::<Vec<_>>();
            movements.sort_by_key(|v| std::cmp::Reverse(v.get("dataHora").and_then(|v| v.as_str()).unwrap_or("")));
            for movement in movements.into_iter().take(10) {
                let date = movement.get("dataHora").and_then(|v| v.as_str()).unwrap_or("");
                let name = movement.get("nome").and_then(|v| v.as_str()).unwrap_or("");
                details.push(format!("{date}: {name}"));
            }
        }
        Some(PublicSearchResult {
            title: format!("{number} — {class} ({tribunal})"),
            // CNJ provides metadata, not a public browser URL for individual API records.
            url: format!("https://www.cnj.jus.br/sistemas/datajud/api-publica/#processo-{number}-{tribunal}-{grade}"),
            description: Some(format!("Classe: {class}\nÓrgão julgador: {court}\nGrau: {grade}")),
            published_at: None,
            source: format!("DataJud / {tribunal}"),
            details: (!details.is_empty()).then(|| details.join("\n")),
        })
    }).take(max).collect()
}

#[derive(Debug, Deserialize)]
struct PncpSearchResponse {
    #[serde(default)]
    items: Vec<serde_json::Value>,
}

fn normalize_pncp_search(
    data: PncpSearchResponse,
    base_url: &str,
    max: usize,
) -> Vec<PublicSearchResult> {
    let base = base_url.trim_end_matches('/');
    data.items
        .into_iter()
        .filter_map(|item| {
            let title_raw = item
                .get("title")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("numero_controle_pncp").and_then(|v| v.as_str()))?
                .trim();
            if title_raw.is_empty() {
                return None;
            }
            let item_url = item
                .get("item_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let url = if item_url.starts_with("http://") || item_url.starts_with("https://") {
                item_url.to_owned()
            } else if let Some(rest) = item_url.strip_prefix("/compras/") {
                format!("{base}/app/editais/{rest}")
            } else if let Some(rest) = item_url.strip_prefix("/contratos/") {
                format!("{base}/app/contratos/{rest}")
            } else if let Some(rest) = item_url.strip_prefix("/atas/") {
                format!("{base}/app/atas/{rest}")
            } else if !item_url.is_empty() {
                format!("{base}/{}", item_url.trim_start_matches('/'))
            } else {
                let id = item.get("id").and_then(|v| v.as_str())?;
                format!("{base}/app/item/{id}")
            };

            let orgao = item
                .get("orgao_nome")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("PNCP");
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);
            let published_at = item
                .get("data_publicacao_pncp")
                .or_else(|| item.get("createdAt"))
                .and_then(|v| v.as_str())
                .and_then(normalize_iso_date);

            let mut details = Vec::new();
            if let Some(doc_type) = item.get("document_type").and_then(|v| v.as_str()) {
                details.push(format!("Documento: {doc_type}"));
            }
            if let Some(mod_nome) = item
                .get("modalidade_licitacao_nome")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                details.push(format!("Modalidade: {mod_nome}"));
            }
            if let Some(sit) = item
                .get("situacao_nome")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                details.push(format!("Situação: {sit}"));
            }
            if let Some(cnpj) = item
                .get("orgao_cnpj")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                details.push(format!("CNPJ do Órgão: {cnpj}"));
            }
            if let Some(fornecedor) = item
                .get("fornecedor_nome")
                .or_else(|| item.get("nome_fornecedor"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                details.push(format!("Fornecedor: {fornecedor}"));
            }
            if let Some(valor) = item.get("valor_global").and_then(|v| v.as_f64()) {
                details.push(format!("Valor Global: R$ {valor:.2}"));
            }
            let mun = item
                .get("municipio_nome")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let uf = item.get("uf").and_then(|v| v.as_str()).unwrap_or("");
            if !mun.is_empty() || !uf.is_empty() {
                details.push(format!("Localidade: {mun}/{uf}"));
            }

            Some(PublicSearchResult {
                title: format!("{title_raw} — {orgao}"),
                url,
                description,
                published_at,
                source: format!("PNCP / {orgao}"),
                details: (!details.is_empty()).then(|| details.join("\n")),
            })
        })
        .take(max)
        .collect()
}

#[derive(Debug, Deserialize)]
struct PncpContratosResponse {
    #[serde(default)]
    data: Vec<serde_json::Value>,
}

fn normalize_pncp_contratos(
    data: PncpContratosResponse,
    base_url: &str,
    max: usize,
) -> Vec<PublicSearchResult> {
    let base = base_url.trim_end_matches('/');
    data.data
        .into_iter()
        .filter_map(|item| {
            let numero = item
                .get("numeroContratoEmpenho")
                .and_then(|v| v.as_str())
                .unwrap_or("S/N");
            let ano = item
                .get("anoContrato")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let seq = item
                .get("sequencialContrato")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let orgao_cnpj = item
                .pointer("/orgaoEntidade/cnpj")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let orgao_nome = item
                .pointer("/orgaoEntidade/razaoSocial")
                .and_then(|v| v.as_str())
                .unwrap_or("Órgão Público");
            let controle = item
                .get("numeroControlePNCP")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let url = if !orgao_cnpj.is_empty() && ano > 0 && seq > 0 {
                format!("{base}/app/contratos/{orgao_cnpj}/{ano}/{seq}")
            } else if !controle.is_empty() {
                format!("{base}/app/contratos/{controle}")
            } else {
                return None;
            };

            let objeto = item
                .get("objetoContrato")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);
            let published_at = item
                .get("dataPublicacaoPncp")
                .or_else(|| item.get("dataAssinatura"))
                .and_then(|v| v.as_str())
                .and_then(normalize_iso_date);

            let mut details = Vec::new();
            if !controle.is_empty() {
                details.push(format!("Controle PNCP: {controle}"));
            }
            if let Some(fornecedor) = item
                .get("nomeRazaoSocialFornecedor")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                let ni = item
                    .get("niFornecedor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if ni.is_empty() {
                    details.push(format!("Fornecedor: {fornecedor}"));
                } else {
                    details.push(format!("Fornecedor: {fornecedor} ({ni})"));
                }
            }
            if let Some(valor) = item.get("valorGlobal").and_then(|v| v.as_f64()) {
                details.push(format!("Valor Global: R$ {valor:.2}"));
            }
            if let Some(vig_ini) = item.get("dataVigenciaInicio").and_then(|v| v.as_str()) {
                let vig_fim = item
                    .get("dataVigenciaFim")
                    .and_then(|v| v.as_str())
                    .unwrap_or("--");
                details.push(format!("Vigência: {vig_ini} a {vig_fim}"));
            }

            Some(PublicSearchResult {
                title: format!("Contrato {numero}/{ano} — {orgao_nome}"),
                url,
                description: objeto,
                published_at,
                source: format!("PNCP / {orgao_nome}"),
                details: (!details.is_empty()).then(|| details.join("\n")),
            })
        })
        .take(max)
        .collect()
}

fn normalize_cgu_payload(
    categoria: &str,
    payload: &serde_json::Value,
    max: usize,
) -> Vec<PublicSearchResult> {
    let items: Vec<&serde_json::Value> = if let Some(arr) = payload.as_array() {
        arr.iter().collect()
    } else if payload.is_object() {
        vec![payload]
    } else {
        return Vec::new();
    };

    items
        .into_iter()
        .filter_map(|item| match categoria {
            "CEIS" | "CNEP" => {
                let id = item
                    .get("id")
                    .and_then(|v| {
                        v.as_i64()
                            .map(|n| n.to_string())
                            .or_else(|| v.as_str().map(str::to_owned))
                    })
                    .or_else(|| {
                        item.get("codigoSancao")
                            .and_then(|v| v.as_str().map(str::to_owned))
                    })?;
                let sancionado = item
                    .pointer("/sancionado/nome")
                    .or_else(|| item.pointer("/pessoa/nome"))
                    .or_else(|| item.get("nomeSancionado"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sancionado");
                let doc = item
                    .pointer("/sancionado/codigoFormatado")
                    .or_else(|| item.pointer("/pessoa/cpfFormatado"))
                    .or_else(|| item.pointer("/pessoa/cnpjFormatado"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tipo_sancao = item
                    .pointer("/tipoSancao/descricaoResumida")
                    .or_else(|| item.pointer("/tipoSancao/descricaoPortal"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sanção administrativa");
                let orgao = item
                    .pointer("/orgaoSancionador/nome")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Órgão Sancionador");
                let data_ini = item
                    .get("dataInicioSancao")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let data_fim = item
                    .get("dataFimSancao")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let processo = item
                    .get("numeroProcesso")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut details = vec![
                    format!("Cadastro: {categoria}"),
                    format!("Órgão Sancionador: {orgao}"),
                ];
                if !doc.is_empty() {
                    details.push(format!("CPF/CNPJ: {doc}"));
                }
                if !processo.is_empty() {
                    details.push(format!("Processo: {processo}"));
                }
                if !data_ini.is_empty() || !data_fim.is_empty() {
                    details.push(format!("Vigência da sanção: {data_ini} até {data_fim}"));
                }

                let slug = categoria.to_ascii_lowercase();
                Some(PublicSearchResult {
                    title: format!("[{categoria}] {sancionado} — {tipo_sancao}"),
                    url: format!("https://portaldatransparencia.gov.br/sancoes/{slug}/{id}"),
                    description: Some(format!("Sanção: {tipo_sancao} aplicada por {orgao}")),
                    published_at: normalize_inlabs_date(data_ini),
                    source: format!("CGU / {categoria}"),
                    details: Some(details.join("\n")),
                })
            }
            "CEPIM" => {
                let id = item.get("id").and_then(|v| {
                    v.as_i64()
                        .map(|n| n.to_string())
                        .or_else(|| v.as_str().map(str::to_owned))
                })?;
                let entidade = item
                    .pointer("/pessoaJuridica/nome")
                    .or_else(|| item.get("nomeEntidade"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Entidade");
                let motivo = item
                    .get("motivo")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Impedimento CEPIM");
                let orgao = item
                    .pointer("/orgaoSuperior/nome")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Governo Federal");
                let convenio = item
                    .pointer("/convenio/numero")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut details = vec![
                    "Cadastro: CEPIM".to_owned(),
                    format!("Órgão Concedente: {orgao}"),
                ];
                if !convenio.is_empty() {
                    details.push(format!("Convênio: {convenio}"));
                }

                Some(PublicSearchResult {
                    title: format!("[CEPIM] {entidade} — {motivo}"),
                    url: format!("https://portaldatransparencia.gov.br/sancoes/cepim/{id}"),
                    description: Some(format!("Motivo do impedimento: {motivo}")),
                    published_at: None,
                    source: "CGU / CEPIM".to_owned(),
                    details: Some(details.join("\n")),
                })
            }
            "SERVIDORES" => {
                let servidor_obj = item.get("servidor").unwrap_or(item);
                let id = servidor_obj
                    .get("id")
                    .or_else(|| servidor_obj.get("idServidorAposentadoPensionista"))
                    .and_then(|v| {
                        v.as_i64()
                            .map(|n| n.to_string())
                            .or_else(|| v.as_str().map(str::to_owned))
                    })?;
                let nome = servidor_obj
                    .pointer("/pessoa/nome")
                    .or_else(|| servidor_obj.get("nome"))
                    .and_then(|v| v.as_str())?;
                let cpf_fmt = servidor_obj
                    .pointer("/pessoa/cpfFormatado")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let orgao = servidor_obj
                    .pointer("/orgaoServidorLotacao/nome")
                    .or_else(|| servidor_obj.pointer("/orgaoServidorExercicio/nome"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Executivo Federal");
                let situacao = servidor_obj
                    .get("situacao")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Ativo");
                let tipo_servidor = servidor_obj
                    .get("tipoServidor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Civil");

                let mut details = vec![
                    format!("Órgão de Lotação: {orgao}"),
                    format!("Situação: {situacao} ({tipo_servidor})"),
                ];
                if !cpf_fmt.is_empty() {
                    details.push(format!("CPF: {cpf_fmt}"));
                }

                Some(PublicSearchResult {
                    title: format!("Servidor Federal: {nome} ({orgao})"),
                    url: format!("https://portaldatransparencia.gov.br/servidores/{id}"),
                    description: Some(format!(
                        "Vínculo público federal em {orgao} · Situação: {situacao}"
                    )),
                    published_at: None,
                    source: "CGU / Servidores".to_owned(),
                    details: Some(details.join("\n")),
                })
            }
            "CONTRATOS" => {
                let id = item.get("id").and_then(|v| {
                    v.as_i64()
                        .map(|n| n.to_string())
                        .or_else(|| v.as_str().map(str::to_owned))
                })?;
                let numero = item.get("numero").and_then(|v| v.as_str()).unwrap_or(&id);
                let objeto = item
                    .get("objeto")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Contrato Federal");
                let orgao = item
                    .pointer("/unidadeGestora/orgaoVinculado/nome")
                    .or_else(|| item.pointer("/unidadeGestora/nome"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unidade Gestora Federal");
                let fornecedor = item
                    .pointer("/fornecedor/nome")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let data_assinatura = item
                    .get("dataAssinatura")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut details = vec![format!("Órgão: {orgao}")];
                if !fornecedor.is_empty() {
                    details.push(format!("Fornecedor: {fornecedor}"));
                }
                if let Some(valor) = item.get("valorInicialCompra").and_then(|v| v.as_f64()) {
                    details.push(format!("Valor Inicial: R$ {valor:.2}"));
                }
                if !data_assinatura.is_empty() {
                    details.push(format!("Assinatura: {data_assinatura}"));
                }

                Some(PublicSearchResult {
                    title: format!("Contrato Federal {numero} — {orgao}"),
                    url: format!("https://portaldatransparencia.gov.br/contratos/{id}"),
                    description: Some(objeto.to_owned()),
                    published_at: normalize_inlabs_date(data_assinatura),
                    source: "CGU / Contratos".to_owned(),
                    details: Some(details.join("\n")),
                })
            }
            _ => None,
        })
        .take(max)
        .collect()
}

fn tse_eleicao_id_por_ano(ano: i32) -> i64 {
    match ano {
        2024 => 2045202024,
        2022 => 2040602022,
        2020 => 2030402020,
        2018 => 2022802018,
        2016 => 2,
        2014 => 680,
        _ => i64::from(ano),
    }
}

fn normalize_tse_candidatos(
    payload: &serde_json::Value,
    ano: i32,
    eleicao_id: i64,
    ue: &str,
    parameter: &ParametroBusca,
    max: usize,
) -> Vec<PublicSearchResult> {
    let candidatos = payload
        .get("candidatos")
        .and_then(|v| v.as_array())
        .or_else(|| payload.as_array());
    let Some(candidatos) = candidatos else {
        return Vec::new();
    };

    let query_lower = remover_acentos_basico(parameter.valor.trim()).to_lowercase();
    let query_digits: String = parameter
        .valor
        .chars()
        .filter(char::is_ascii_digit)
        .collect();

    candidatos
        .iter()
        .filter_map(|cand| {
            let id = cand
                .get("id")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_owned)))?;
            let nome_completo = cand
                .get("nomeCompleto")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let nome_urna = cand
                .get("nomeUrna")
                .and_then(|v| v.as_str())
                .unwrap_or(nome_completo)
                .trim();
            if nome_completo.is_empty() && nome_urna.is_empty() {
                return None;
            }
            let cpf_cand: String = cand
                .get("cpf")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            let cnpj_cand: String = cand
                .get("cnpjcampanha")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .filter(char::is_ascii_digit)
                .collect();

            let corresponde = match parameter.tipo.as_str() {
                "CPF" => !query_digits.is_empty() && cpf_cand == query_digits,
                "CNPJ" => !query_digits.is_empty() && cnpj_cand == query_digits,
                _ => {
                    let completo_norm = remover_acentos_basico(nome_completo).to_lowercase();
                    let urna_norm = remover_acentos_basico(nome_urna).to_lowercase();
                    completo_norm.contains(&query_lower) || urna_norm.contains(&query_lower)
                }
            };
            if !corresponde {
                return None;
            }

            let cargo = cand
                .pointer("/cargo/nome")
                .or_else(|| cand.get("nomeCargo"))
                .and_then(|v| v.as_str())
                .unwrap_or("Candidatura");
            let partido = cand
                .pointer("/partido/sigla")
                .or_else(|| cand.get("siglaPartido"))
                .and_then(|v| v.as_str())
                .unwrap_or("--");
            let numero = cand
                .get("numero")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_owned)))
                .unwrap_or_default();
            let situacao = cand
                .get("descricaoSituacao")
                .or_else(|| cand.get("descricaoSituacaoCandidatura"))
                .and_then(|v| v.as_str())
                .unwrap_or("Registrado");
            let coligacao = cand
                .get("nomeColigacao")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let mut details = vec![
                format!("Nome completo: {nome_completo}"),
                format!("Nome de urna: {nome_urna}"),
                format!("Cargo: {cargo} ({ue})"),
                format!("Partido: {partido} · Número: {numero}"),
                format!("Situação: {situacao}"),
            ];
            if !coligacao.is_empty() {
                details.push(format!("Coligação/Federação: {coligacao}"));
            }
            if !cnpj_cand.is_empty() {
                details.push(format!("CNPJ de Campanha: {cnpj_cand}"));
            }
            if let Some(bens) = cand.get("totalDeBens").and_then(|v| v.as_f64()) {
                details.push(format!("Total de Bens Declarados: R$ {bens:.2}"));
            }

            Some(PublicSearchResult {
                title: format!(
                    "{nome_urna} ({nome_completo}) — {cargo} ({partido}/{ue}) · Eleições {ano}"
                ),
                url: format!(
                    "https://divulgacandcontas.tse.jus.br/divulga/#/candidato/{ano}/{eleicao_id}/{ue}/{id}"
                ),
                description: Some(format!(
                    "Candidatura a {cargo} ({ue}) pelo {partido} nas Eleições {ano} · Situação: {situacao}"
                )),
                published_at: Some(format!("{ano}-10-01")),
                source: format!("DivulgaCandContas / TSE ({ano})"),
                details: Some(details.join("\n")),
            })
        })
        .take(max)
        .collect()
}

fn normalize_iso_date(value: &str) -> Option<String> {
    let prefix = value.trim().get(0..10)?;
    if NaiveDate::parse_from_str(prefix, "%Y-%m-%d").is_ok() {
        return Some(prefix.to_owned());
    }
    normalize_inlabs_date(value.trim())
}

fn remover_acentos_basico(texto: &str) -> String {
    texto
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            other => other,
        })
        .collect()
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
    fn datajud_routes_only_valid_process_numbers() {
        for (value, expected) in [
            ("0000832-35.2018.4.01.3202", "trf1"),
            ("0000000-00.2024.8.26.0000", "tjsp"),
            ("0000000-00.2024.5.03.0000", "trt3"),
            ("0000000-00.2024.6.07.0000", "tre-dft"),
            ("0000000-00.2024.7.01.0000", "stm"),
        ] {
            let (number, tribunal) = datajud_query(value).unwrap();
            assert_eq!(number.len(), 20);
            assert_eq!(tribunal, expected);
        }
        for invalid in [
            "Maria Silva",
            "12345678901",
            "0000000-00.2024.8.99.0000",
            "0000000-00.2024.8.26.0000/injection",
        ] {
            assert!(datajud_query(invalid).is_err());
        }
        assert_eq!(
            SearchProvider::parse("datajud"),
            Ok(SearchProvider::Datajud)
        );
    }

    #[test]
    fn datajud_normalizes_metadata_and_distinguishes_instances() {
        let data = serde_json::from_value(serde_json::json!({"hits":{"hits":[
            {"_source":{"numeroProcesso":"00000000020248260000","tribunal":"TJSP","grau":"G1","classe":{"nome":"Procedimento comum"},"orgaoJulgador":{"nome":"Vara cível"}}},
            {"_source":{"numeroProcesso":"00000000020248260000","tribunal":"TJSP","grau":"G2"}}
        ]}})).unwrap();
        let results = normalize_datajud(data, 10);
        assert_eq!(results.len(), 2);
        assert!(results[0].title.contains("Procedimento comum"));
        assert!(
            results[0]
                .description
                .as_ref()
                .unwrap()
                .contains("Vara cível")
        );
        assert_ne!(results[0].url, results[1].url);
        assert!(serde_json::from_str::<DatajudResponse>("{}").is_err());
        assert!(
            normalize_datajud(serde_json::from_str(r#"{"hits":{"hits":[]}}"#).unwrap(), 10)
                .is_empty()
        );
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

    #[test]
    fn normalizes_pncp_search_and_contracts() {
        let search_data: PncpSearchResponse = serde_json::from_value(serde_json::json!({
            "items": [{
                "title": "Edital nº 10/2026",
                "description": "Aquisição de equipamentos de informática",
                "item_url": "/compras/00394460000141/2026/10",
                "orgao_nome": "Ministério da Fazenda",
                "orgao_cnpj": "00394460000141",
                "modalidade_licitacao_nome": "Pregão Eletrônico",
                "situacao_nome": "Divulgada no PNCP",
                "valor_global": 125000.50,
                "data_publicacao_pncp": "2026-03-15T10:00:00",
                "municipio_nome": "Brasília",
                "uf": "DF"
            }]
        }))
        .unwrap();
        let search_results = normalize_pncp_search(search_data, "https://pncp.gov.br", 10);
        assert_eq!(search_results.len(), 1);
        assert_eq!(
            search_results[0].url,
            "https://pncp.gov.br/app/editais/00394460000141/2026/10"
        );
        assert_eq!(
            search_results[0].published_at.as_deref(),
            Some("2026-03-15")
        );
        assert!(
            search_results[0]
                .details
                .as_deref()
                .unwrap()
                .contains("Pregão Eletrônico")
        );

        let contratos_data: PncpContratosResponse = serde_json::from_value(serde_json::json!({
            "data": [{
                "numeroContratoEmpenho": "45",
                "anoContrato": 2026,
                "sequencialContrato": 7,
                "numeroControlePNCP": "00394460000141-2-000007/2026",
                "objetoContrato": "Prestação de serviços de nuvem",
                "valorGlobal": 98000.0,
                "nomeRazaoSocialFornecedor": "Tech Brasil LTDA",
                "niFornecedor": "12345678000199",
                "dataPublicacaoPncp": "2026-04-01",
                "orgaoEntidade": {
                    "cnpj": "00394460000141",
                    "razaoSocial": "Ministério da Gestão"
                }
            }]
        }))
        .unwrap();
        let contratos_results = normalize_pncp_contratos(contratos_data, "https://pncp.gov.br", 10);
        assert_eq!(contratos_results.len(), 1);
        assert_eq!(
            contratos_results[0].url,
            "https://pncp.gov.br/app/contratos/00394460000141/2026/7"
        );
        assert!(
            contratos_results[0]
                .details
                .as_deref()
                .unwrap()
                .contains("Tech Brasil LTDA")
        );
    }

    #[test]
    fn normalizes_cgu_ceis_and_servidores() {
        let ceis = serde_json::json!([{
            "id": 9988,
            "sancionado": {"nome": "Empresa Exemplo S/A", "codigoFormatado": "12.345.678/0001-99"},
            "tipoSancao": {"descricaoResumida": "Declaração de Inidoneidade"},
            "orgaoSancionador": {"nome": "Controladoria-Geral da União"},
            "dataInicioSancao": "10/02/2026",
            "dataFimSancao": "10/02/2028",
            "numeroProcesso": "00190.123456/2025-00"
        }]);
        let results = normalize_cgu_payload("CEIS", &ceis, 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("[CEIS] Empresa Exemplo S/A"));
        assert_eq!(
            results[0].url,
            "https://portaldatransparencia.gov.br/sancoes/ceis/9988"
        );

        let servidores = serde_json::json!([{
            "servidor": {
                "id": 4455,
                "pessoa": {"nome": "Maria Silva", "cpfFormatado": "***.456.789-**"},
                "orgaoServidorLotacao": {"nome": "Ministério da Saúde"},
                "situacao": "Ativo Permanente",
                "tipoServidor": "Civil"
            }
        }]);
        let srv_results = normalize_cgu_payload("SERVIDORES", &servidores, 10);
        assert_eq!(srv_results.len(), 1);
        assert!(srv_results[0].title.contains("Maria Silva"));
        assert_eq!(
            srv_results[0].url,
            "https://portaldatransparencia.gov.br/servidores/4455"
        );
    }

    #[test]
    fn normalizes_tse_divulgacandcontas() {
        let payload = serde_json::json!({
            "candidatos": [{
                "id": 280001234567i64,
                "nomeCompleto": "MARIA DA SILVA SANTOS",
                "nomeUrna": "MARIA SILVA",
                "numero": 1234,
                "cargo": {"nome": "Deputado Federal"},
                "partido": {"sigla": "ABC"},
                "descricaoSituacao": "Deferido",
                "cnpjcampanha": "45678901000123",
                "totalDeBens": 350000.0
            }]
        });
        let param = parameter("TSE");
        let results = normalize_tse_candidatos(&payload, 2022, 2040602022, "SP", &param, 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("MARIA SILVA"));
        assert_eq!(
            results[0].url,
            "https://divulgacandcontas.tse.jus.br/divulga/#/candidato/2022/2040602022/SP/280001234567"
        );
    }

    #[test]
    fn expands_compatible_providers_based_on_type_and_configuration() {
        let mut cfg = OsintConfiguracao::default();
        let cnpj_param = ParametroBusca {
            id: 10,
            pessoa_id: 1,
            tipo: "CNPJ".to_owned(),
            valor: "00.394.460/0001-41".to_owned(),
            provider: "COMPATIVEIS".to_owned(),
            ativo: true,
        };
        // Sem chave CGU ou SearXNG, expande para PNCP, TSE e QUERIDO_DIARIO
        let expandidos = cfg.expandir_parametros(vec![cnpj_param.clone()], false);
        let providers: Vec<&str> = expandidos.iter().map(|p| p.provider.as_str()).collect();
        assert_eq!(providers, vec!["PNCP", "TSE", "QUERIDO_DIARIO"]);

        // Com chave CGU configurada, inclui CGU automaticamente
        cfg.cgu_api_key = Some("chave-cgu-valida".to_owned());
        let expandidos_cgu = cfg.expandir_parametros(vec![cnpj_param], false);
        let providers_cgu: Vec<&str> = expandidos_cgu.iter().map(|p| p.provider.as_str()).collect();
        assert_eq!(providers_cgu, vec!["PNCP", "CGU", "TSE", "QUERIDO_DIARIO"]);
    }
}
