use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

use crate::{
    AppState,
    error::AppError,
    models::{Contato, ContatoInput, PessoaDetalhe, PessoaInput, PessoaResumo, PessoaUpdateInput},
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/", get(listar_pessoas).post(criar_pessoa))
        .route(
            "/{id}",
            get(obter_pessoa)
                .put(atualizar_pessoa)
                .delete(excluir_pessoa),
        )
        .route(
            "/{pessoa_id}/contatos",
            get(listar_contatos).post(criar_contato),
        )
        .route(
            "/contatos/{id}",
            get(obter_contato)
                .put(atualizar_contato)
                .delete(excluir_contato),
        )
}

async fn listar_pessoas(
    State(state): State<AppState>,
) -> Result<Json<Vec<PessoaResumo>>, AppError> {
    let pessoas = sqlx::query_as::<_, PessoaResumo>(
        "SELECT p.id, p.nome, p.categoria_id, c.nome_categoria, c.cor_hex, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.data_cadastro \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         ORDER BY p.nome COLLATE NOCASE",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(pessoas))
}

async fn obter_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<PessoaDetalhe>, AppError> {
    Ok(Json(buscar_pessoa_detalhe(&state, id).await?))
}

async fn criar_pessoa(
    State(state): State<AppState>,
    Json(input): Json<PessoaInput>,
) -> Result<(StatusCode, Json<PessoaDetalhe>), AppError> {
    validar_nome(&input.nome)?;
    for contato in &input.contatos {
        validar_contato(contato)?;
    }

    let mut tx = state.pool.begin().await?;
    let pessoa_id: i64 =
        sqlx::query_scalar("INSERT INTO pessoa (nome, categoria_id) VALUES (?, ?) RETURNING id")
            .bind(input.nome.trim())
            .bind(input.categoria_id)
            .fetch_one(&mut *tx)
            .await?;

    for contato in input.contatos {
        sqlx::query("INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?)")
            .bind(pessoa_id)
            .bind(contato.tipo_contato_id)
            .bind(contato.valor.trim())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    let pessoa = buscar_pessoa_detalhe(&state, pessoa_id).await?;
    Ok((StatusCode::CREATED, Json(pessoa)))
}

async fn atualizar_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<PessoaUpdateInput>,
) -> Result<Json<PessoaDetalhe>, AppError> {
    validar_nome(&input.nome)?;
    let resultado = sqlx::query("UPDATE pessoa SET nome = ?, categoria_id = ? WHERE id = ?")
        .bind(input.nome.trim())
        .bind(input.categoria_id)
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(Json(buscar_pessoa_detalhe(&state, id).await?))
}

async fn excluir_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM pessoa WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn listar_contatos(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<Contato>>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let contatos = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato \
         WHERE pessoa_id = ? ORDER BY id",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(contatos))
}

async fn obter_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Contato>, AppError> {
    let contato = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    Ok(Json(contato))
}

async fn criar_contato(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    Json(input): Json<ContatoInput>,
) -> Result<(StatusCode, Json<Contato>), AppError> {
    validar_contato(&input)?;
    let contato = sqlx::query_as::<_, Contato>(
        "INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?) \
         RETURNING id, pessoa_id, tipo_contato_id, valor",
    )
    .bind(pessoa_id)
    .bind(input.tipo_contato_id)
    .bind(input.valor.trim())
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(contato)))
}

async fn atualizar_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<ContatoInput>,
) -> Result<Json<Contato>, AppError> {
    validar_contato(&input)?;
    let contato = sqlx::query_as::<_, Contato>(
        "UPDATE contato SET tipo_contato_id = ?, valor = ? WHERE id = ? \
         RETURNING id, pessoa_id, tipo_contato_id, valor",
    )
    .bind(input.tipo_contato_id)
    .bind(input.valor.trim())
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    Ok(Json(contato))
}

async fn excluir_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM contato WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("contato"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_pessoa_detalhe(state: &AppState, id: i64) -> Result<PessoaDetalhe, AppError> {
    let pessoa = sqlx::query_as::<_, PessoaResumo>(
        "SELECT p.id, p.nome, p.categoria_id, c.nome_categoria, c.cor_hex, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.data_cadastro \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE p.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("pessoa"))?;
    let contatos = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato \
         WHERE pessoa_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(PessoaDetalhe { pessoa, contatos })
}

async fn garantir_pessoa(state: &AppState, id: i64) -> Result<(), AppError> {
    let existe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ?)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if !existe {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(())
}

fn validar_nome(nome: &str) -> Result<(), AppError> {
    if nome.trim().is_empty() {
        return Err(AppError::BadRequest("nome é obrigatório".to_owned()));
    }
    Ok(())
}

fn validar_contato(input: &ContatoInput) -> Result<(), AppError> {
    if input.tipo_contato_id <= 0 || input.valor.trim().is_empty() {
        return Err(AppError::BadRequest(
            "tipo_contato_id e valor são obrigatórios".to_owned(),
        ));
    }
    Ok(())
}
