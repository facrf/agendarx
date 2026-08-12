use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("não autenticado")]
    Unauthorized,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("arquivo excede o limite permitido")]
    PayloadTooLarge,
    #[error("{0}")]
    ServiceUnavailable(String),
    #[error("erro interno")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    erro: String,
}

impl AppError {
    pub fn configuracao(mensagem: impl Into<String>) -> Self {
        Self::Internal(mensagem.into())
    }

    pub fn interno(mensagem: impl Into<String>) -> Self {
        Self::Internal(mensagem.into())
    }

    pub fn nao_encontrado(recurso: &str) -> Self {
        Self::NotFound(format!("{recurso} não encontrado"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        if status.is_server_error() {
            tracing::error!(erro = %self, detalhes = ?self, "requisição falhou");
        }
        (
            status,
            Json(ErrorBody {
                erro: self.to_string(),
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(erro: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &erro {
            if db.is_unique_violation() {
                return Self::Conflict("registro duplicado".to_owned());
            }
            if db.is_foreign_key_violation() {
                return Self::Conflict(
                    "operação impedida por um relacionamento existente ou inválido".to_owned(),
                );
            }
            if db.message().contains("CHECK constraint failed") {
                return Self::BadRequest("dados não atendem às regras de validação".to_owned());
            }
        }
        Self::Internal(erro.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(erro: sqlx::migrate::MigrateError) -> Self {
        Self::Internal(erro.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(erro: std::io::Error) -> Self {
        Self::Internal(erro.to_string())
    }
}
