use std::{env, net::SocketAddr, path::PathBuf, str::FromStr};

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct Config {
    pub endereco: SocketAddr,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_minutos: i64,
    pub max_upload_bytes: usize,
    pub frontend_dir: PathBuf,
    pub cookie_secure: bool,
    pub admin_login: Option<String>,
    pub admin_password: Option<String>,
    pub searxng_url: Option<String>,
    pub osint_timeout_seconds: u64,
    pub osint_max_results: usize,
    pub osint_max_pdf_bytes: usize,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let _ = dotenvy::dotenv();

        let endereco = parse_env("SERVER_ADDR", "0.0.0.0:3000")?;
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/agendarx.db?mode=rwc".to_owned());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET ausente; usando segredo apenas para desenvolvimento. Configure-o em produção"
            );
            "desenvolvimento-altere-este-segredo-32-chars".to_owned()
        });
        if jwt_secret.len() < 32 {
            return Err(AppError::configuracao(
                "JWT_SECRET deve possuir pelo menos 32 caracteres",
            ));
        }

        let admin_login = env::var("ADMIN_LOGIN")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let admin_password = env::var("ADMIN_PASSWORD")
            .ok()
            .filter(|v| !v.trim().is_empty());
        if admin_login.is_some() != admin_password.is_some() {
            return Err(AppError::configuracao(
                "ADMIN_LOGIN e ADMIN_PASSWORD devem ser informados juntos",
            ));
        }

        let max_upload_bytes = parse_env("MAX_UPLOAD_BYTES", "26214400")?;
        let osint_timeout_seconds = parse_env("OSINT_TIMEOUT_SECONDS", "20")?;
        let osint_max_results = parse_env("OSINT_MAX_RESULTS", "15")?;
        let osint_max_pdf_bytes = parse_env("OSINT_MAX_PDF_BYTES", "20971520")?;
        if osint_timeout_seconds == 0 || osint_max_results == 0 || osint_max_results > 100 {
            return Err(AppError::configuracao(
                "OSINT_TIMEOUT_SECONDS deve ser maior que zero e OSINT_MAX_RESULTS deve estar entre 1 e 100",
            ));
        }
        if osint_max_pdf_bytes == 0 || osint_max_pdf_bytes > max_upload_bytes {
            return Err(AppError::configuracao(
                "OSINT_MAX_PDF_BYTES deve ser maior que zero e não pode exceder MAX_UPLOAD_BYTES",
            ));
        }

        Ok(Self {
            endereco,
            database_url,
            jwt_secret,
            jwt_ttl_minutos: parse_env("JWT_TTL_MINUTES", "480")?,
            max_upload_bytes,
            frontend_dir: env::var("FRONTEND_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("frontend/dist")),
            cookie_secure: parse_bool_env("COOKIE_SECURE", false)?,
            admin_login,
            admin_password,
            searxng_url: env::var("SEARXNG_URL")
                .ok()
                .map(|valor| valor.trim().trim_end_matches('/').to_owned())
                .filter(|valor| !valor.is_empty()),
            osint_timeout_seconds,
            osint_max_results,
            osint_max_pdf_bytes,
        })
    }
}

fn parse_env<T>(nome: &str, padrao: &str) -> Result<T, AppError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let valor = env::var(nome).unwrap_or_else(|_| padrao.to_owned());
    valor
        .parse()
        .map_err(|erro| AppError::configuracao(format!("valor inválido em {nome}: {erro}")))
}

fn parse_bool_env(nome: &str, padrao: bool) -> Result<bool, AppError> {
    match env::var(nome) {
        Ok(valor) => match valor.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "sim" => Ok(true),
            "0" | "false" | "no" | "nao" | "não" => Ok(false),
            _ => Err(AppError::configuracao(format!(
                "valor booleano inválido em {nome}"
            ))),
        },
        Err(_) => Ok(padrao),
    }
}
