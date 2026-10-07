use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration as StdDuration, Instant},
};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};

use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::SqlitePool;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    middleware::auth::{Claims, SessaoAutenticada},
    models::{
        CredenciaisInput, LoginInput, LoginResponse, MensagemResponse, Usuario, UsuarioSessao,
    },
};

const MAX_ICONE_ADMIN_BYTES: usize = 2 * 1024 * 1024;
const MAX_CORPO_LOGIN_BYTES: usize = 16 * 1024;
const MAX_TENTATIVAS_POR_LOGIN: u32 = 10;
const MAX_TENTATIVAS_GLOBAIS: u32 = 120;
const MAX_VERIFICACOES_SIMULTANEAS: usize = 4;
const JANELA_TENTATIVAS: StdDuration = StdDuration::from_secs(60);
// PHC válido com o mesmo custo do Argon2::default(). O resultado da verificação
// jamais autentica um usuário ausente; evita pular o trabalho de derivação.
const HASH_USUARIO_AUSENTE: &str = "$argon2id$v=19$m=19456,t=2,p=1$c2VndXJhbmNhLWxvZ2luLXYx$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

#[derive(Clone)]
pub struct AuthRuntime {
    tentativas: Arc<Mutex<Tentativas>>,
    verificacoes: Arc<Semaphore>,
}

impl Default for AuthRuntime {
    fn default() -> Self {
        Self {
            tentativas: Arc::new(Mutex::new(Tentativas::default())),
            verificacoes: Arc::new(Semaphore::new(MAX_VERIFICACOES_SIMULTANEAS)),
        }
    }
}

#[derive(Default)]
struct Tentativas {
    global: Option<JanelaTentativas>,
    por_login: HashMap<String, JanelaTentativas>,
}

struct JanelaTentativas {
    inicio: Instant,
    quantidade: u32,
}

impl JanelaTentativas {
    fn restante(&self, agora: Instant) -> u64 {
        JANELA_TENTATIVAS
            .saturating_sub(agora.saturating_duration_since(self.inicio))
            .as_secs()
            .saturating_add(1)
    }
}

impl Tentativas {
    fn registrar(&mut self, login: &str, agora: Instant) -> Result<(), AppError> {
        self.por_login
            .retain(|_, janela| agora.saturating_duration_since(janela.inicio) < JANELA_TENTATIVAS);
        let global = self.global.get_or_insert(JanelaTentativas {
            inicio: agora,
            quantidade: 0,
        });
        if agora.saturating_duration_since(global.inicio) >= JANELA_TENTATIVAS {
            *global = JanelaTentativas {
                inicio: agora,
                quantidade: 0,
            };
        }
        if global.quantidade >= MAX_TENTATIVAS_GLOBAIS {
            return Err(AppError::TooManyRequests(global.restante(agora)));
        }
        global.quantidade += 1;
        // O limite global também limita o crescimento deste mapa, inclusive
        // quando os pedidos alternam nomes de usuários inexistentes.
        let janela = self
            .por_login
            .entry(login.to_owned())
            .or_insert(JanelaTentativas {
                inicio: agora,
                quantidade: 0,
            });
        if janela.quantidade >= MAX_TENTATIVAS_POR_LOGIN {
            return Err(AppError::TooManyRequests(janela.restante(agora)));
        }
        janela.quantidade += 1;
        Ok(())
    }
}

impl AuthRuntime {
    fn admitir(&self, login: &str) -> Result<OwnedSemaphorePermit, AppError> {
        self.tentativas
            .lock()
            .map_err(|_| AppError::interno("falha no controle de tentativas"))?
            .registrar(login, Instant::now())?;
        self.verificacoes
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::TooManyRequests(1))
    }
}

pub fn rotas_publicas() -> Router<AppState> {
    Router::new().route(
        "/login",
        post(login).layer(DefaultBodyLimit::max(MAX_CORPO_LOGIN_BYTES)),
    )
}

pub fn rotas_protegidas() -> Router<AppState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/sessao", get(verificar_sessao))
        .route(
            "/credenciais",
            put(atualizar_credenciais).layer(DefaultBodyLimit::max(MAX_CORPO_LOGIN_BYTES)),
        )
        .route(
            "/icone",
            get(obter_icone_admin)
                .put(atualizar_icone_admin)
                .layer(DefaultBodyLimit::max(MAX_ICONE_ADMIN_BYTES))
                .delete(excluir_icone_admin),
        )
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
    if login.chars().count() > 64 || input.senha.chars().count() > 1024 {
        return Err(AppError::BadRequest(
            "usuário ou senha excedem o tamanho permitido".to_owned(),
        ));
    }
    let verificacao = state.auth_runtime.admitir(login)?;

    let usuario = sqlx::query_as::<_, Usuario>(
        "SELECT id, login, perfil, senha_hash, icone_admin_blob, icone_admin_mime_type, \
                icone_admin_atualizado_em, data_criacao \
         FROM usuario WHERE login = ?",
    )
    .bind(login)
    .fetch_optional(&state.pool)
    .await?;

    let senha = input.senha;
    let hash = usuario
        .as_ref()
        .map_or(HASH_USUARIO_AUSENTE, |u| u.senha_hash.as_str())
        .to_owned();
    let senha_valida = tokio::task::spawn_blocking(move || {
        // Mover a permissão para o trabalho bloqueante mantém o limite mesmo
        // se o cliente abandonar a requisição durante o Argon2.
        let _verificacao = verificacao;
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
    let usuario = usuario.ok_or(AppError::Unauthorized)?;

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

    registrar_sessao(
        &state.pool,
        &usuario,
        &sessao_id,
        agora.timestamp(),
        expira_em.timestamp(),
    )
    .await?;

    let resposta = LoginResponse {
        token: token.clone(),
        token_tipo: "Bearer",
        expira_em: expira_em.timestamp(),
        usuario: UsuarioSessao {
            id: usuario.id,
            login: usuario.login,
            perfil: usuario.perfil,
            tem_icone: usuario.icone_admin_blob.is_some(),
            icone_atualizado_em: usuario.icone_admin_atualizado_em,
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

async fn registrar_sessao(
    pool: &SqlitePool,
    usuario: &Usuario,
    id: &str,
    agora: i64,
    expira: i64,
) -> Result<(), AppError> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let inserida = sqlx::query("INSERT INTO sessao(id, usuario_id, expira_em) SELECT ?, id, ? FROM usuario WHERE id = ? AND login = ? AND senha_hash = ?")
        .bind(id).bind(expira).bind(usuario.id).bind(&usuario.login).bind(&usuario.senha_hash).execute(&mut *tx).await?;
    if inserida.rows_affected() != 1 {
        return Err(AppError::Unauthorized);
    }
    sqlx::query("DELETE FROM sessao WHERE expira_em <= ?")
        .bind(agora)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod seguranca_tests {
    use super::*;

    #[test]
    fn tentativas_expiram_e_concorrencia_fica_limitada() {
        let runtime = AuthRuntime::default();
        let permits: Vec<_> = (0..MAX_VERIFICACOES_SIMULTANEAS)
            .map(|i| runtime.admitir(&format!("user{i}")).unwrap())
            .collect();
        assert!(matches!(
            runtime.admitir("extra"),
            Err(AppError::TooManyRequests(_))
        ));
        drop(permits);
        assert!(runtime.admitir("extra").is_ok());
        let mut tentativas = Tentativas::default();
        let now = Instant::now();
        for _ in 0..MAX_TENTATIVAS_POR_LOGIN {
            tentativas.registrar("user", now).unwrap();
        }
        assert!(tentativas.registrar("user", now).is_err());
        assert!(
            tentativas
                .registrar("user", now + JANELA_TENTATIVAS)
                .is_ok()
        );
        assert!(PasswordHash::new(HASH_USUARIO_AUSENTE).is_ok());
    }

    #[tokio::test]
    async fn senha_validada_antes_da_rotacao_nao_emite_sessao_depois() {
        let mut config = crate::config::Config::from_env().unwrap();
        config.database_url = "sqlite::memory:".into();
        config.admin_login = Some("race-test".into());
        config.admin_password = Some("senha-segura-teste".into());
        let pool = crate::db::conectar(&config).await.unwrap();
        let usuario: Usuario = sqlx::query_as("SELECT * FROM usuario WHERE login='race-test'")
            .fetch_one(&pool)
            .await
            .unwrap();
        registrar_sessao(&pool, &usuario, "antes", 0, i64::MAX)
            .await
            .unwrap();
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query("UPDATE usuario SET senha_hash='hash-novo' WHERE id=?")
            .bind(usuario.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("DELETE FROM sessao WHERE usuario_id=?")
            .bind(usuario.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert!(matches!(
            registrar_sessao(&pool, &usuario, "atrasada", 0, i64::MAX).await,
            Err(AppError::Unauthorized)
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessao")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }
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
        "SELECT id, login, perfil, senha_hash, icone_admin_blob, icone_admin_mime_type, \
                icone_admin_atualizado_em, data_criacao \
         FROM usuario WHERE id = ?",
    )
    .bind(sessao.usuario.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let verificacao = state.auth_runtime.admitir(&usuario.login)?;
    let senha_atual = input.senha_atual;
    let hash_atual = usuario.senha_hash.clone();
    let senha_valida = tokio::task::spawn_blocking(move || {
        let _verificacao = verificacao;
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
        let verificacao = state
            .auth_runtime
            .verificacoes
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::TooManyRequests(1))?;
        Some(
            tokio::task::spawn_blocking(move || {
                let _verificacao = verificacao;
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

    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let atualizada = sqlx::query("UPDATE usuario SET login = ?, senha_hash = ? WHERE id = ? AND login = ? AND senha_hash = ?")
        .bind(login).bind(novo_hash.as_deref().unwrap_or(&usuario.senha_hash)).bind(usuario.id)
        .bind(&usuario.login).bind(&usuario.senha_hash).execute(&mut *tx).await?;
    if atualizada.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "as credenciais mudaram; entre novamente".into(),
        ));
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

async fn obter_icone_admin(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Response, AppError> {
    let icone = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT icone_admin_blob, icone_admin_mime_type FROM usuario \
         WHERE id = ? AND icone_admin_blob IS NOT NULL AND icone_admin_mime_type IS NOT NULL",
    )
    .bind(sessao.usuario.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("ícone do administrador"))?;

    let mut resposta = Response::new(Body::from(icone.0));
    let headers = resposta.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&icone.1)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache, no-store, must-revalidate"),
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

async fn atualizar_icone_admin(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    conteudo: Bytes,
) -> Result<Json<UsuarioSessao>, AppError> {
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("o ícone está vazio".to_owned()));
    }
    if conteudo.len() > MAX_ICONE_ADMIN_BYTES || conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }
    let mime_type = infer::get(&conteudo)
        .map(|tipo| tipo.mime_type())
        .filter(|mime| mime.starts_with("image/"))
        .ok_or_else(|| {
            AppError::BadRequest(
                "use uma imagem raster reconhecida (PNG, JPEG, WebP, GIF ou ICO)".to_owned(),
            )
        })?;

    let atualizado_em: String = sqlx::query_scalar(
        "UPDATE usuario SET icone_admin_blob = ?, icone_admin_mime_type = ?, \
                icone_admin_atualizado_em = CURRENT_TIMESTAMP \
         WHERE id = ? RETURNING icone_admin_atualizado_em",
    )
    .bind(conteudo.as_ref())
    .bind(mime_type)
    .bind(sessao.usuario.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(UsuarioSessao {
        id: sessao.usuario.id,
        login: sessao.usuario.login,
        perfil: sessao.usuario.perfil,
        tem_icone: true,
        icone_atualizado_em: Some(atualizado_em),
    }))
}

async fn excluir_icone_admin(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<StatusCode, AppError> {
    sqlx::query(
        "UPDATE usuario SET icone_admin_blob = NULL, icone_admin_mime_type = NULL, \
                icone_admin_atualizado_em = NULL WHERE id = ?",
    )
    .bind(sessao.usuario.id)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) fn validar_novo_login(login: &str) -> Result<(), AppError> {
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

pub(super) fn validar_nova_senha(senha: String) -> Result<String, AppError> {
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
