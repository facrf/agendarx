use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::{get, put},
};

use crate::{AppState, error::AppError, models::IdentidadeVisualResponse};

const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;
const ICONE_PADRAO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="34" fill="#e7654f"/><text x="64" y="86" text-anchor="middle" font-family="system-ui,sans-serif" font-size="76" font-weight="700" fill="white">A</text></svg>"##;

pub fn rotas_protegidas() -> Router<AppState> {
    Router::new()
        .route("/identidade", get(obter_identidade))
        .route("/icone", put(atualizar_icone).delete(excluir_icone))
}

pub fn rotas_publicas() -> Router<AppState> {
    Router::new().route("/icone", get(obter_icone))
}

async fn obter_identidade(
    State(state): State<AppState>,
) -> Result<Json<IdentidadeVisualResponse>, AppError> {
    let atualizado_em =
        sqlx::query_scalar::<_, String>("SELECT atualizado_em FROM identidade_visual WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    Ok(Json(IdentidadeVisualResponse {
        tem_icone: atualizado_em.is_some(),
        atualizado_em,
    }))
}

async fn atualizar_icone(
    State(state): State<AppState>,
    conteudo: Bytes,
) -> Result<Json<IdentidadeVisualResponse>, AppError> {
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("o ícone está vazio".to_owned()));
    }
    if conteudo.len() > MAX_ICON_BYTES || conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }

    let mime_detectado = infer::get(&conteudo).map(|tipo| tipo.mime_type());
    if !mime_detectado.is_some_and(|mime| mime.starts_with("image/")) {
        return Err(AppError::BadRequest(
            "use uma imagem raster reconhecida (PNG, JPEG, WebP, GIF ou ICO)".to_owned(),
        ));
    }
    let mime_type = mime_detectado.unwrap_or("application/octet-stream");

    let atualizado_em: String = sqlx::query_scalar(
        "INSERT INTO identidade_visual (id, icone_blob, mime_type, atualizado_em) \
         VALUES (1, ?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(id) DO UPDATE SET \
            icone_blob = excluded.icone_blob, \
            mime_type = excluded.mime_type, \
            atualizado_em = CURRENT_TIMESTAMP \
         RETURNING atualizado_em",
    )
    .bind(conteudo.as_ref())
    .bind(mime_type)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(IdentidadeVisualResponse {
        tem_icone: true,
        atualizado_em: Some(atualizado_em),
    }))
}

async fn excluir_icone(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM identidade_visual WHERE id = 1")
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn obter_icone(State(state): State<AppState>) -> Result<Response, AppError> {
    let personalizado = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT icone_blob, mime_type FROM identidade_visual WHERE id = 1",
    )
    .fetch_optional(&state.pool)
    .await?;
    let (conteudo, mime_type) = personalizado
        .unwrap_or_else(|| (ICONE_PADRAO.as_bytes().to_vec(), "image/svg+xml".to_owned()));

    let mut resposta = Response::new(Body::from(conteudo));
    let headers = resposta.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("sandbox; default-src 'none'"),
    );
    Ok(resposta)
}

#[cfg(test)]
mod tests {
    use super::ICONE_PADRAO;

    #[test]
    fn icone_padrao_e_svg_autocontido() {
        assert!(ICONE_PADRAO.starts_with("<svg"));
        assert!(ICONE_PADRAO.contains(">A</text>"));
        assert!(!ICONE_PADRAO.contains("<script"));
    }
}
