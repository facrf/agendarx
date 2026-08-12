use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Extension, Json, Router,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    middleware::auth::{Claims, SessaoAutenticada},
    models::{
        CredenciaisInput, LoginInput, LoginResponse, MensagemResponse, Usuario, UsuarioSessao,
    },
};

pub fn rotas_publicas() -> Router<AppState> {
    Router::new().route("/login", post(login))
}

pub fn rotas_protegidas() -> Router<AppState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/sessao", get(verificar_sessao))
        .route("/credenciais", put(atualizar_credenciais))
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

async fn atualizar_credenciais(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<CredenciaisInput>,
) -> Result<Response, AppError> {
    let login = input.login.trim();
    validar_novo_login(login)?;
    if input.senha_atual.is_empty() || input.senha_atual.chars().count() > 1024 {
        return Err(AppError::BadRequest("informe a senha atual".to_owned()));
    }

    let usuario = sqlx::query_as::<_, Usuario>(
        "SELECT id, login, senha_hash, data_criacao FROM usuario WHERE id = ?",
    )
    .bind(sessao.usuario.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let senha_atual = input.senha_atual;
    let hash_atual = usuario.senha_hash.clone();
    let senha_valida = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash_atual).ok().is_some_and(|hash| {
            Argon2::default()
                .verify_password(senha_atual.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await
    .map_err(|_| AppError::interno("falha ao validar a senha atual"))?;
    if !senha_valida {
        return Err(AppError::BadRequest("senha atual incorreta".to_owned()));
    }

    let nova_senha = input
        .nova_senha
        .filter(|senha| !senha.is_empty())
        .map(validar_nova_senha)
        .transpose()?;
    if login == usuario.login && nova_senha.is_none() {
        return Err(AppError::BadRequest(
            "altere o usuário ou informe uma nova senha".to_owned(),
        ));
    }

    let novo_hash = if let Some(nova_senha) = nova_senha {
        Some(
            tokio::task::spawn_blocking(move || {
                let salt = SaltString::generate(&mut OsRng);
                Argon2::default()
                    .hash_password(nova_senha.as_bytes(), &salt)
                    .map(|hash| hash.to_string())
            })
            .await
            .map_err(|_| AppError::interno("falha ao proteger a nova senha"))?
            .map_err(|_| AppError::interno("falha ao proteger a nova senha"))?,
        )
    } else {
        None
    };

    let mut tx = state.pool.begin().await?;
    if let Some(novo_hash) = novo_hash {
        sqlx::query("UPDATE usuario SET login = ?, senha_hash = ? WHERE id = ?")
            .bind(login)
            .bind(novo_hash)
            .bind(usuario.id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE usuario SET login = ? WHERE id = ?")
            .bind(login)
            .bind(usuario.id)
            .execute(&mut *tx)
            .await?;
    }
    // Claims já emitidos carregam o login antigo; revogar todas as sessões também
    // encerra dispositivos que ainda conheçam a senha anterior.
    sqlx::query("DELETE FROM sessao WHERE usuario_id = ?")
        .bind(usuario.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let cookie = cookie_sessao("", 0, state.config.cookie_secure);
    Ok((
        [(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie)
                .map_err(|_| AppError::interno("falha ao remover cookie de sessão"))?,
        )],
        Json(MensagemResponse {
            mensagem: "credenciais atualizadas; entre novamente".to_owned(),
        }),
    )
        .into_response())
}

fn validar_novo_login(login: &str) -> Result<(), AppError> {
    let tamanho = login.chars().count();
    if !(3..=64).contains(&tamanho) {
        return Err(AppError::BadRequest(
            "o usuário deve possuir entre 3 e 64 caracteres".to_owned(),
        ));
    }
    if login.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "o usuário contém caracteres inválidos".to_owned(),
        ));
    }
    Ok(())
}

fn validar_nova_senha(senha: String) -> Result<String, AppError> {
    let tamanho = senha.chars().count();
    if !(8..=1024).contains(&tamanho) {
        return Err(AppError::BadRequest(
            "a nova senha deve possuir entre 8 e 1024 caracteres".to_owned(),
        ));
    }
    Ok(senha)
}

fn cookie_sessao(token: &str, max_age: i64, secure: bool) -> String {
    format!(
        "agendarx_token={token}; Path=/api; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::{cookie_sessao, validar_nova_senha, validar_novo_login};

    #[test]
    fn valida_novas_credenciais() {
        assert!(validar_novo_login("admin").is_ok());
        assert!(validar_novo_login("ab").is_err());
        assert!(validar_nova_senha("senha-forte".to_owned()).is_ok());
        assert!(validar_nova_senha("curta".to_owned()).is_err());
    }

    #[test]
    fn cookie_removido_expira_imediatamente() {
        let cookie = cookie_sessao("", 0, true);
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("; Secure"));
    }
}
