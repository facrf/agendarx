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
        "SELECT p.id, p.nome, p.categoria_id, p.descricao, c.nome_categoria, c.cor_hex, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.pessoa_juridica, p.data_cadastro \
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
    validar_descricao(input.descricao.as_deref())?;
    for contato in &input.contatos {
        validar_contato(contato)?;
    }

    let mut tx = state.pool.begin().await?;
    let pessoa_id: i64 = sqlx::query_scalar(
        "INSERT INTO pessoa (nome, categoria_id, descricao, pessoa_juridica) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(input.nome.trim())
    .bind(input.categoria_id)
    .bind(normalizar_descricao(input.descricao))
    .bind(input.pessoa_juridica)
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
    validar_descricao(input.descricao.as_deref())?;
    let resultado =
        sqlx::query("UPDATE pessoa SET nome = ?, categoria_id = ?, descricao = ?, pessoa_juridica = ? WHERE id = ?")
            .bind(input.nome.trim())
            .bind(input.categoria_id)
            .bind(normalizar_descricao(input.descricao))
            .bind(input.pessoa_juridica)
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
        "SELECT p.id, p.nome, p.categoria_id, p.descricao, c.nome_categoria, c.cor_hex, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.pessoa_juridica, p.data_cadastro \
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

fn validar_descricao(descricao: Option<&str>) -> Result<(), AppError> {
    if descricao.is_some_and(|valor| valor.chars().count() > 5_000) {
        return Err(AppError::BadRequest(
            "a descrição deve ter no máximo 5000 caracteres".to_owned(),
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

fn validar_contato(input: &ContatoInput) -> Result<(), AppError> {
    if input.tipo_contato_id <= 0 || input.valor.trim().is_empty() {
        return Err(AppError::BadRequest(
            "tipo_contato_id e valor são obrigatórios".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{normalizar_descricao, validar_descricao};

    #[test]
    fn normaliza_descricao_vazia_e_remove_espacos() {
        assert_eq!(normalizar_descricao(Some("   ".to_owned())), None);
        assert_eq!(
            normalizar_descricao(Some("  Perfil detalhado  ".to_owned())),
            Some("Perfil detalhado".to_owned())
        );
    }

    #[test]
    fn limita_descricao_por_caracteres() {
        assert!(validar_descricao(Some(&"á".repeat(5_000))).is_ok());
        assert!(validar_descricao(Some(&"á".repeat(5_001))).is_err());
    }
}
