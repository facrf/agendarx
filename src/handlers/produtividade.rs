use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/etiquetas", get(listar_etiquetas).post(criar_etiqueta))
        .route(
            "/etiquetas/{id}",
            axum::routing::put(atualizar_etiqueta).delete(excluir_etiqueta),
        )
        .route(
            "/pessoas/{id}/etiquetas",
            get(etiquetas_pessoa).put(salvar_etiquetas_pessoa),
        )
        .route(
            "/pessoas/{id}/favorito",
            axum::routing::put(salvar_favorito),
        )
        .route("/lixeira", get(listar_lixeira))
        .route(
            "/lixeira/{id}/restaurar",
            axum::routing::post(restaurar_pessoa),
        )
        .route(
            "/lixeira/{id}",
            axum::routing::delete(excluir_definitivamente),
        )
        .route("/auditoria", get(listar_auditoria))
        .route(
            "/grafo/posicoes/{layout}",
            get(obter_posicoes).put(salvar_posicoes),
        )
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Etiqueta {
    pub id: i64,
    pub nome: String,
    pub cor_hex: String,
}

#[derive(Deserialize)]
struct EtiquetaInput {
    nome: String,
    cor_hex: String,
}

#[derive(Deserialize)]
struct EtiquetasPessoaInput {
    #[serde(default)]
    etiquetas_ids: Vec<i64>,
}

#[derive(Deserialize)]
struct FavoritoInput {
    favorito: bool,
}

#[derive(Serialize, sqlx::FromRow)]
struct PessoaLixeira {
    id: i64,
    nome: String,
    excluida_em: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct AuditoriaItem {
    id: i64,
    usuario_login: String,
    acao: String,
    recurso: String,
    status_http: i64,
    data_evento: String,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
struct PosicaoGrafo {
    pessoa_id: i64,
    x: f64,
    y: f64,
}

async fn listar_etiquetas(State(state): State<AppState>) -> Result<Json<Vec<Etiqueta>>, AppError> {
    Ok(Json(
        sqlx::query_as("SELECT id, nome, cor_hex FROM etiqueta ORDER BY nome COLLATE NOCASE")
            .fetch_all(&state.pool)
            .await?,
    ))
}

fn validar_etiqueta(input: &EtiquetaInput) -> Result<(), AppError> {
    let nome = input.nome.trim();
    let cor = input.cor_hex.as_bytes();
    if nome.is_empty() || nome.chars().count() > 50 {
        return Err(AppError::BadRequest(
            "o nome da etiqueta deve ter entre 1 e 50 caracteres".into(),
        ));
    }
    if cor.len() != 7 || cor[0] != b'#' || !cor[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err(AppError::BadRequest(
            "cor_hex deve usar o formato #RRGGBB".into(),
        ));
    }
    Ok(())
}

async fn criar_etiqueta(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<EtiquetaInput>,
) -> Result<(StatusCode, Json<Etiqueta>), AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    validar_etiqueta(&input)?;
    let item = sqlx::query_as(
        "INSERT INTO etiqueta (nome, cor_hex) VALUES (?, ?) RETURNING id, nome, cor_hex",
    )
    .bind(input.nome.trim())
    .bind(input.cor_hex.to_ascii_uppercase())
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn atualizar_etiqueta(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(input): Json<EtiquetaInput>,
) -> Result<Json<Etiqueta>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    validar_etiqueta(&input)?;
    let item = sqlx::query_as(
        "UPDATE etiqueta SET nome = ?, cor_hex = ? WHERE id = ? RETURNING id, nome, cor_hex",
    )
    .bind(input.nome.trim())
    .bind(input.cor_hex.to_ascii_uppercase())
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("etiqueta"))?;
    Ok(Json(item))
}

async fn excluir_etiqueta(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    let result = sqlx::query("DELETE FROM etiqueta WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("etiqueta"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn etiquetas_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Etiqueta>>, AppError> {
    Ok(Json(sqlx::query_as("SELECT e.id, e.nome, e.cor_hex FROM etiqueta e JOIN pessoa_etiqueta pe ON pe.etiqueta_id = e.id WHERE pe.pessoa_id = ? ORDER BY e.nome COLLATE NOCASE")
        .bind(id).fetch_all(&state.pool).await?))
}

async fn salvar_etiquetas_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<EtiquetasPessoaInput>,
) -> Result<Json<Vec<Etiqueta>>, AppError> {
    let mut ids = input.etiquetas_ids;
    ids.sort_unstable();
    ids.dedup();
    let mut tx = state.pool.begin().await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ? AND excluida_em IS NULL)",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if !exists {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    for etiqueta_id in &ids {
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM etiqueta WHERE id = ?)")
            .bind(etiqueta_id)
            .fetch_one(&mut *tx)
            .await?;
        if !valid {
            return Err(AppError::BadRequest(
                "uma ou mais etiquetas não existem".into(),
            ));
        }
    }
    sqlx::query("DELETE FROM pessoa_etiqueta WHERE pessoa_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for etiqueta_id in ids {
        sqlx::query("INSERT INTO pessoa_etiqueta (pessoa_id, etiqueta_id) VALUES (?, ?)")
            .bind(id)
            .bind(etiqueta_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    etiquetas_pessoa(State(state), Path(id)).await
}

async fn salvar_favorito(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(input): Json<FavoritoInput>,
) -> Result<StatusCode, AppError> {
    if input.favorito {
        sqlx::query("INSERT INTO pessoa_favorita (usuario_id, pessoa_id) SELECT ?, id FROM pessoa WHERE id = ? AND excluida_em IS NULL ON CONFLICT DO NOTHING").bind(sessao.usuario.id).bind(id).execute(&state.pool).await?;
    } else {
        sqlx::query("DELETE FROM pessoa_favorita WHERE usuario_id = ? AND pessoa_id = ?")
            .bind(sessao.usuario.id)
            .bind(id)
            .execute(&state.pool)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn listar_lixeira(
    State(state): State<AppState>,
) -> Result<Json<Vec<PessoaLixeira>>, AppError> {
    Ok(Json(sqlx::query_as("SELECT id, nome, excluida_em FROM pessoa WHERE excluida_em IS NOT NULL ORDER BY excluida_em DESC").fetch_all(&state.pool).await?))
}

async fn restaurar_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query(
        "UPDATE pessoa SET excluida_em = NULL WHERE id = ? AND excluida_em IS NOT NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa na lixeira"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn excluir_definitivamente(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    let result = sqlx::query("DELETE FROM pessoa WHERE id = ? AND excluida_em IS NOT NULL")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa na lixeira"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn listar_auditoria(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<Vec<AuditoriaItem>>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(Json(sqlx::query_as("SELECT id, usuario_login, acao, recurso, status_http, data_evento FROM auditoria ORDER BY id DESC LIMIT 500").fetch_all(&state.pool).await?))
}

fn validar_layout(layout: &str) -> Result<(), AppError> {
    if matches!(layout, "force" | "hierarchical") {
        Ok(())
    } else {
        Err(AppError::BadRequest("layout inválido".into()))
    }
}

async fn obter_posicoes(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(layout): Path<String>,
) -> Result<Json<Vec<PosicaoGrafo>>, AppError> {
    validar_layout(&layout)?;
    Ok(Json(
        sqlx::query_as(
            "SELECT pessoa_id, x, y FROM posicao_grafo WHERE usuario_id = ? AND layout = ?",
        )
        .bind(sessao.usuario.id)
        .bind(layout)
        .fetch_all(&state.pool)
        .await?,
    ))
}

async fn salvar_posicoes(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(layout): Path<String>,
    Json(posicoes): Json<Vec<PosicaoGrafo>>,
) -> Result<StatusCode, AppError> {
    validar_layout(&layout)?;
    if posicoes.len() > 10_000
        || posicoes
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err(AppError::BadRequest("posições inválidas".into()));
    }
    let mut tx = state.pool.begin().await?;
    for p in posicoes {
        sqlx::query("INSERT INTO posicao_grafo (usuario_id, layout, pessoa_id, x, y) VALUES (?, ?, ?, ?, ?) ON CONFLICT(usuario_id, layout, pessoa_id) DO UPDATE SET x=excluded.x, y=excluded.y, data_atualizacao=CURRENT_TIMESTAMP")
        .bind(sessao.usuario.id).bind(&layout).bind(p.pessoa_id).bind(p.x).bind(p.y).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
