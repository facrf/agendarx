use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

use crate::{
    AppState,
    error::AppError,
    models::{GrafoEdge, GrafoNode, GrafoResponse, PessoaVinculo, VinculoInput},
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/", get(listar_vinculos).post(criar_vinculo))
        .route("/grafo", get(obter_grafo))
        .route(
            "/{id}",
            get(obter_vinculo)
                .put(atualizar_vinculo)
                .delete(excluir_vinculo),
        )
}

async fn listar_vinculos(
    State(state): State<AppState>,
) -> Result<Json<Vec<PessoaVinculo>>, AppError> {
    let vinculos = sqlx::query_as::<_, PessoaVinculo>(
        "SELECT id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao \
         FROM pessoa_vinculo ORDER BY data_criacao DESC, id DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(vinculos))
}

async fn obter_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<PessoaVinculo>, AppError> {
    let vinculo = buscar_vinculo(&state, id).await?;
    Ok(Json(vinculo))
}

async fn criar_vinculo(
    State(state): State<AppState>,
    Json(input): Json<VinculoInput>,
) -> Result<(StatusCode, Json<PessoaVinculo>), AppError> {
    validar_vinculo(&input)?;
    let vinculo = sqlx::query_as::<_, PessoaVinculo>(
        "INSERT INTO pessoa_vinculo \
            (pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao) \
         VALUES (?, ?, ?, ?) \
         RETURNING id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao",
    )
    .bind(input.pessoa_origem_id)
    .bind(input.pessoa_destino_id)
    .bind(input.tipo_vinculo.trim())
    .bind(normalizar_descricao(input.descricao))
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(vinculo)))
}

async fn atualizar_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<VinculoInput>,
) -> Result<Json<PessoaVinculo>, AppError> {
    validar_vinculo(&input)?;
    let vinculo = sqlx::query_as::<_, PessoaVinculo>(
        "UPDATE pessoa_vinculo SET \
            pessoa_origem_id = ?, pessoa_destino_id = ?, tipo_vinculo = ?, descricao = ? \
         WHERE id = ? \
         RETURNING id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao",
    )
    .bind(input.pessoa_origem_id)
    .bind(input.pessoa_destino_id)
    .bind(input.tipo_vinculo.trim())
    .bind(normalizar_descricao(input.descricao))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("vínculo"))?;
    Ok(Json(vinculo))
}

async fn excluir_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM pessoa_vinculo WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("vínculo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn obter_grafo(State(state): State<AppState>) -> Result<Json<GrafoResponse>, AppError> {
    let nodes = sqlx::query_as::<_, GrafoNode>(
        "SELECT p.id, p.nome AS label, COALESCE(c.cor_hex, '#86A6A3') AS color, \
                CASE WHEN p.foto_principal IS NOT NULL \
                    THEN '/api/dossie/pessoas/' || p.id || '/foto' \
                    ELSE NULL \
                END AS foto_url, \
                c.nome_categoria AS categoria \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         ORDER BY p.nome COLLATE NOCASE",
    )
    .fetch_all(&state.pool)
    .await?;
    let edges = sqlx::query_as::<_, GrafoEdge>(
        "SELECT id, pessoa_origem_id AS source, pessoa_destino_id AS target, \
                tipo_vinculo AS label, descricao \
         FROM pessoa_vinculo ORDER BY id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(GrafoResponse { nodes, edges }))
}

async fn buscar_vinculo(state: &AppState, id: i64) -> Result<PessoaVinculo, AppError> {
    sqlx::query_as::<_, PessoaVinculo>(
        "SELECT id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao \
         FROM pessoa_vinculo WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("vínculo"))
}

fn validar_vinculo(input: &VinculoInput) -> Result<(), AppError> {
    if input.pessoa_origem_id <= 0 || input.pessoa_destino_id <= 0 {
        return Err(AppError::BadRequest(
            "pessoa_origem_id e pessoa_destino_id são obrigatórios".to_owned(),
        ));
    }
    if input.pessoa_origem_id == input.pessoa_destino_id {
        return Err(AppError::BadRequest(
            "uma pessoa não pode ter vínculo consigo mesma".to_owned(),
        ));
    }
    if input.tipo_vinculo.trim().is_empty() {
        return Err(AppError::BadRequest(
            "tipo_vinculo é obrigatório".to_owned(),
        ));
    }
    Ok(())
}

fn normalizar_descricao(descricao: Option<String>) -> Option<String> {
    descricao.and_then(|valor| {
        let valor = valor.trim().to_owned();
        (!valor.is_empty()).then_some(valor)
    })
}
