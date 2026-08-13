use axum::{
    extract::{Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::AppError, models::UsuarioSessao};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub login: String,
    pub jti: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct SessaoAutenticada {
    pub usuario: UsuarioSessao,
    pub sessao_id: String,
    pub expira_em: i64,
}

pub async fn exigir_autenticacao(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extrair_token(request.headers()).ok_or(AppError::Unauthorized)?;
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "sub"]);

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;

    let usuario_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized)?;
    let agora = Utc::now().timestamp();
    let usuario_sessao = sqlx::query_as::<_, (String, bool, Option<String>)>(
        "SELECT u.login, (u.icone_admin_blob IS NOT NULL), u.icone_admin_atualizado_em \
         FROM sessao s JOIN usuario u ON u.id = s.usuario_id \
         WHERE s.id = ? AND s.usuario_id = ? AND s.expira_em > ? AND u.login = ?",
    )
    .bind(&claims.jti)
    .bind(usuario_id)
    .bind(agora)
    .bind(&claims.login)
    .fetch_optional(&state.pool)
    .await?;

    let (login, tem_icone, icone_atualizado_em) = usuario_sessao.ok_or(AppError::Unauthorized)?;

    request.extensions_mut().insert(SessaoAutenticada {
        usuario: UsuarioSessao {
            id: usuario_id,
            login,
            tem_icone,
            icone_atualizado_em,
        },
        sessao_id: claims.jti,
        expira_em: claims.exp as i64,
    });
    Ok(next.run(request).await)
}

fn extrair_token(headers: &HeaderMap) -> Option<&str> {
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|valor| valor.to_str().ok())
        .and_then(|valor| valor.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
    {
        return Some(token);
    }

    headers
        .get(header::COOKIE)
        .and_then(|valor| valor.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (nome, valor) = cookie.trim().split_once('=')?;
                (nome == "agendarx_token").then_some(valor)
            })
        })
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::extrair_token;

    #[test]
    fn aceita_bearer_antes_do_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer abc"),
        );
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("outro=x; agendarx_token=def"),
        );
        assert_eq!(extrair_token(&headers), Some("abc"));
    }
}
