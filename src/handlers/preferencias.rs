use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
#[derive(Default, Serialize, Deserialize, sqlx::FromRow)]
struct Preferencias {
    silencioso_inicio: Option<String>,
    silencioso_fim: Option<String>,
}
pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/lembretes", get(obter).put(salvar))
        .route("/lembretes/{id}/adiar", post(adiar))
}
async fn obter(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<Preferencias>, AppError> {
    Ok(Json(
        sqlx::query_as(
            "SELECT silencioso_inicio,silencioso_fim FROM preferencia_lembrete WHERE usuario_id=?",
        )
        .bind(sessao.usuario.id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or_default(),
    ))
}
async fn salvar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<Preferencias>,
) -> Result<Json<Preferencias>, AppError> {
    for v in [&input.silencioso_inicio, &input.silencioso_fim]
        .into_iter()
        .flatten()
    {
        if v.len() != 5 || chrono::NaiveTime::parse_from_str(v, "%H:%M").is_err() {
            return Err(AppError::BadRequest("Use horários no formato HH:MM".into()));
        }
    }
    if input.silencioso_inicio.is_some() != input.silencioso_fim.is_some()
        || input.silencioso_inicio.is_some() && input.silencioso_inicio == input.silencioso_fim
    {
        return Err(AppError::BadRequest(
            "Informe início e fim diferentes, ou desative o horário silencioso".into(),
        ));
    }
    sqlx::query("INSERT INTO preferencia_lembrete(usuario_id,silencioso_inicio,silencioso_fim) VALUES(?,?,?) ON CONFLICT(usuario_id) DO UPDATE SET silencioso_inicio=excluded.silencioso_inicio,silencioso_fim=excluded.silencioso_fim").bind(sessao.usuario.id).bind(&input.silencioso_inicio).bind(&input.silencioso_fim).execute(&state.pool).await?;
    Ok(Json(input))
}
#[derive(Deserialize)]
struct AdiarInput {
    minutos: i64,
    versao: i64,
}
async fn adiar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(input): Json<AdiarInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    if ![5, 15, 30, 60, 1440].contains(&input.minutos) {
        return Err(AppError::BadRequest(
            "Escolha 5, 15, 30, 60 ou 1440 minutos".into(),
        ));
    }
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    super::revisoes::verificar(&mut tx, "tarefa", id, sessao.usuario.id, Some(input.versao))
        .await?;
    let ate = (chrono::Utc::now() + chrono::Duration::minutes(input.minutos))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let resultado=sqlx::query("UPDATE tarefa_calendario SET lembrete_adiado_ate=?,lembrete_dispensado_em=NULL WHERE id=? AND usuario_id=? AND status<>'CONCLUIDA' AND lembrete_minutos IS NOT NULL").bind(&ate).bind(id).bind(sessao.usuario.id).execute(&mut *tx).await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::Conflict(
            "Esta tarefa não tem um lembrete disponível para adiar".into(),
        ));
    }
    tx.commit().await?;
    Ok(Json(serde_json::json!({"adiado_ate":ate})))
}
