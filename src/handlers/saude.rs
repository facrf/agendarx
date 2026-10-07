use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};
use axum::{
    Extension, Json, Router,
    extract::State,
    routing::{get, post},
};
pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/", get(obter))
        .route("/backup/verificar", post(verificar))
}
async fn obter(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<serde_json::Value>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    let armazenamento = super::configuracoes::obter_diagnostico_armazenamento(State(state.clone()))
        .await?
        .0;
    let backup = super::backup::resumo_saude(&state).await?;
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT estado,COUNT(*) FROM osint_trabalho GROUP BY estado")
            .fetch_all(&state.pool)
            .await?;
    let falhas:Vec<(String,i64,String,Option<String>,String)>=sqlx::query_as("SELECT j.id,j.pessoa_id,p.nome,j.erro,j.atualizado_em FROM osint_trabalho j JOIN pessoa p ON p.id=j.pessoa_id WHERE j.estado='erro' ORDER BY j.atualizado_em DESC LIMIT 10").fetch_all(&state.pool).await?;
    Ok(Json(
        serde_json::json!({"armazenamento":armazenamento,"backup":backup,"pesquisas_por_estado":rows,"falhas_pesquisa":falhas}),
    ))
}
async fn verificar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<serde_json::Value>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(Json(super::backup::verificar_ultimo_backup(&state).await?))
}
