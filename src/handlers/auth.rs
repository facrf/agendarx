use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Extension, Json, Router,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    middleware::auth::{Claims, SessaoAutenticada},
    models::{LoginInput, LoginResponse, Usuario, UsuarioSessao},
};

pub fn rotas_publicas() -> Router<AppState> {
    Router::new().route("/login", post(login))
}

pub fn rotas_protegidas() -> Router<AppState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/sessao", get(verificar_sessao))
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Response, AppError> {
    let login = input.login.trim();
    if login.is_empty() || input.senha.is_empty() {
        return Err(AppError::BadRequest(
            "login e senha são obrigatórios".to_owned(),
        ));
    }

    let usuario = sqlx::query_as::<_, Usuario>(
        "SELECT id, login, senha_hash, data_criacao FROM usuario WHERE login = ?",
    )
    .bind(login)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let senha = input.senha;
    let hash = usuario.senha_hash.clone();
    let senha_valida = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).ok().is_some_and(|hash| {
            Argon2::default()
                .verify_password(senha.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await
    .map_err(|_| AppError::interno("falha ao validar credenciais"))?;
    if !senha_valida {
        return Err(AppError::Unauthorized);
    }

    let agora = Utc::now();
    let expira_em = agora + Duration::minutes(state.config.jwt_ttl_minutos);
    let sessao_id = Uuid::new_v4().to_string();
    let claims = Claims {
        sub: usuario.id.to_string(),
        login: usuario.login.clone(),
        jti: sessao_id.clone(),
        iat: agora.timestamp() as usize,
        exp: expira_em.timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::interno("falha ao criar token de sessão"))?;

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM sessao WHERE expira_em <= ?")
        .bind(agora.timestamp())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO sessao (id, usuario_id, expira_em) VALUES (?, ?, ?)")
        .bind(&sessao_id)
        .bind(usuario.id)
        .bind(expira_em.timestamp())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let resposta = LoginResponse {
        token: token.clone(),
        token_tipo: "Bearer",
        expira_em: expira_em.timestamp(),
        usuario: UsuarioSessao {
            id: usuario.id,
            login: usuario.login,
        },
    };
    let cookie = cookie_sessao(
        &token,
        state.config.jwt_ttl_minutos * 60,
        state.config.cookie_secure,
    );

    Ok((
        [(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie)
                .map_err(|_| AppError::interno("falha ao criar cookie de sessão"))?,
        )],
        Json(resposta),
    )
        .into_response())
}

async fn logout(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Response, AppError> {
    sqlx::query("DELETE FROM sessao WHERE id = ?")
        .bind(sessao.sessao_id)
        .execute(&state.pool)
        .await?;

    let cookie = cookie_sessao("", 0, state.config.cookie_secure);
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie)
                .map_err(|_| AppError::interno("falha ao remover cookie de sessão"))?,
        )],
    )
        .into_response())
}

async fn verificar_sessao(
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "autenticado": true,
        "usuario": sessao.usuario,
        "expira_em": sessao.expira_em,
    }))
}

fn cookie_sessao(token: &str, max_age: i64, secure: bool) -> String {
    format!(
        "agendarx_token={token}; Path=/api; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    )
}
