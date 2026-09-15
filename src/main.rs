mod config;
mod db;
mod error;
mod handlers;
#[cfg(test)]
mod integration_tests;
mod middleware;
mod models;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, OriginalUri, Request},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::get,
};
use serde_json::json;
use sqlx::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::Config, error::AppError};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub backup_runtime: handlers::backup::BackupRuntime,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agendarx=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let pool = db::conectar(&config).await?;
    let state = AppState {
        pool,
        config: config.clone(),
        backup_runtime: handlers::backup::BackupRuntime::default(),
    };
    handlers::backup::iniciar_rotina(state.clone());

    let app = construir_app(state);
    let listener = tokio::net::TcpListener::bind(config.endereco).await?;
    tracing::info!(endereco = %config.endereco, "AgendarX iniciado");
    axum::serve(listener, app)
        .with_graceful_shutdown(encerrar_graciosamente())
        .await?;
    Ok(())
}

fn construir_app(state: AppState) -> Router {
    let config = &state.config;
    // Bytes e Multipart têm um limite próprio (2 MiB por padrão), além do tower-http.
    // A margem cobre os delimitadores; handlers validam o tamanho real do arquivo.
    let limite_corpo_requisicao = usize::try_from(config.task_storage_quota_bytes)
        .unwrap_or(config.max_upload_bytes)
        .max(config.max_upload_bytes)
        .saturating_add(1024 * 1024);

    let protegidas = Router::new()
        .nest("/api/auth", handlers::auth::rotas_protegidas())
        .nest("/api/configuracoes", handlers::configuracoes::rotas())
        .nest("/api/calendario", handlers::calendario::rotas())
        .nest("/api/pessoas", handlers::pessoas::rotas())
        .nest("/api/produtividade", handlers::produtividade::rotas())
        .nest("/api/dossie", handlers::dossie::rotas())
        .nest("/api/osint", handlers::osint::rotas())
        .nest("/api/vinculos", handlers::vinculos::rotas())
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::auth::exigir_autenticacao,
        ))
        .route_layer(from_fn_with_state(
            state.clone(),
            handlers::backup::aguardar_manutencao,
        ));

    let index_frontend = config.frontend_dir.join("index.html");
    let arquivos_frontend =
        ServeDir::new(&config.frontend_dir).fallback(ServeFile::new(index_frontend));

    Router::new()
        .route("/health", get(health))
        .nest("/api/auth", handlers::auth::rotas_publicas())
        .nest("/api/identidade", handlers::identidade::rotas_publicas())
        .merge(protegidas)
        .layer(DefaultBodyLimit::max(limite_corpo_requisicao))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .fallback(move |OriginalUri(uri): OriginalUri, request: Request| {
            let mut arquivos_frontend = arquivos_frontend.clone();
            async move {
                if uri.path() == "/api" || uri.path().starts_with("/api/") {
                    return AppError::NotFound("Rota da API não encontrada".to_owned())
                        .into_response();
                }
                match arquivos_frontend.try_call(request).await {
                    Ok(response) => response.into_response(),
                    Err(error) => AppError::from(error).into_response(),
                }
            }
        })
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "servico": "agendarx" }))
}

async fn encerrar_graciosamente() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("falha ao instalar sinal Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("falha ao instalar sinal SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
