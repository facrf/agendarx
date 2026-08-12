mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;

use axum::{Json, Router, middleware::from_fn_with_state, routing::get};
use serde_json::json;
use sqlx::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::{compression::CompressionLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::Config, error::AppError};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
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
    };

    let protegidas = Router::new()
        .nest("/api/auth", handlers::auth::rotas_protegidas())
        .nest("/api/configuracoes", handlers::configuracoes::rotas())
        .nest("/api/pessoas", handlers::pessoas::rotas())
        .nest("/api/dossie", handlers::dossie::rotas())
        .nest("/api/osint", handlers::osint::rotas())
        .nest("/api/vinculos", handlers::vinculos::rotas())
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::auth::exigir_autenticacao,
        ));

    let index_frontend = config.frontend_dir.join("index.html");
    let arquivos_frontend =
        ServeDir::new(&config.frontend_dir).fallback(ServeFile::new(index_frontend));

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api/auth", handlers::auth::rotas_publicas())
        .nest("/api/identidade", handlers::identidade::rotas_publicas())
        .merge(protegidas)
        .layer(RequestBodyLimitLayer::new(config.max_upload_bytes))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .fallback_service(arquivos_frontend)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.endereco).await?;
    tracing::info!(endereco = %config.endereco, "AgendarX iniciado");
    axum::serve(listener, app)
        .with_graceful_shutdown(encerrar_graciosamente())
        .await?;
    Ok(())
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
