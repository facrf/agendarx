use std::path::Path as FsPath;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

use crate::{
    AppState,
    error::AppError,
    models::{
        CategoriaInput, CategoriaPessoa, ConsumoUsuarioAdminResponse,
        DiagnosticoArmazenamentoResponse, TipoMeioContato, TipoMeioContatoInput,
    },
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/categorias", get(listar_categorias).post(criar_categoria))
        .route(
            "/categorias/{id}",
            get(obter_categoria)
                .put(atualizar_categoria)
                .delete(excluir_categoria),
        )
        .route("/tipos-contato", get(listar_tipos).post(criar_tipo_contato))
        .route("/admin/diagnostico-armazenamento", get(obter_diagnostico_armazenamento))
        .route(
            "/tipos-contato/{id}",
            get(obter_tipo_contato)
                .put(atualizar_tipo_contato)
                .delete(excluir_tipo_contato),
        )
        .merge(super::identidade::rotas_protegidas())
        .merge(super::intercambio::rotas())
}

/// Nesta versão de usuário único, toda sessão autenticada é a sessão administrativa.
/// Ao introduzir papéis, este endpoint deve exigir explicitamente o papel ADMIN.
async fn obter_diagnostico_armazenamento(
    State(state): State<AppState>,
) -> Result<Json<DiagnosticoArmazenamentoResponse>, AppError> {
    let (dossie_bytes, dossie_total): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(tamanho_bytes), 0), COUNT(*) FROM anexo_dossie",
    )
    .fetch_one(&state.pool)
    .await?;
    let (vinculos_bytes, vinculos_total): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(tamanho_bytes), 0), COUNT(*) FROM anexo_vinculo",
    )
    .fetch_one(&state.pool)
    .await?;
    let (tarefas_bytes, tarefas_anexos_total): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(tamanho_bytes), 0), COUNT(*) FROM anexo_tarefa_calendario",
    )
    .fetch_one(&state.pool)
    .await?;
    let pessoas_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pessoa")
        .fetch_one(&state.pool)
        .await?;
    let usuarios = sqlx::query_as::<_, ConsumoUsuarioAdminResponse>(
        "SELECT u.id, u.login, COUNT(DISTINCT t.id) AS tarefas_total, COUNT(a.id) AS anexos_tarefas_total, \
                COALESCE(SUM(a.tamanho_bytes), 0) AS armazenamento_tarefas_bytes \
         FROM usuario u \
         LEFT JOIN tarefa_calendario t ON t.usuario_id = u.id \
         LEFT JOIN anexo_tarefa_calendario a ON a.tarefa_id = t.id \
         GROUP BY u.id, u.login ORDER BY armazenamento_tarefas_bytes DESC, u.login COLLATE NOCASE",
    )
    .fetch_all(&state.pool)
    .await?;

    let banco_bytes = tamanho_banco(&state.config.database_url);
    Ok(Json(DiagnosticoArmazenamentoResponse {
        banco_bytes,
        dossie_bytes,
        vinculos_bytes,
        tarefas_bytes,
        midia_total_bytes: dossie_bytes + vinculos_bytes + tarefas_bytes,
        anexos_total: dossie_total + vinculos_total + tarefas_anexos_total,
        pessoas_total,
        limite_usuario_tarefas_bytes: state.config.task_storage_quota_bytes,
        max_arquivo_bytes: state.config.max_upload_bytes as i64,
        usuarios,
    }))
}

fn tamanho_banco(database_url: &str) -> i64 {
    let Some(caminho) = database_url
        .strip_prefix("sqlite://")
        .and_then(|valor| valor.split('?').next())
    else {
        return 0;
    };
    std::fs::metadata(FsPath::new(caminho))
        .ok()
        .and_then(|metadata| i64::try_from(metadata.len()).ok())
        .unwrap_or(0)
}

async fn listar_categorias(
    State(state): State<AppState>,
) -> Result<Json<Vec<CategoriaPessoa>>, AppError> {
    let itens = sqlx::query_as::<_, CategoriaPessoa>(
        "SELECT id, nome_categoria, cor_hex FROM categoria_pessoa ORDER BY nome_categoria",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(itens))
}

async fn obter_categoria(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<CategoriaPessoa>, AppError> {
    Ok(Json(buscar_categoria(&state, id).await?))
}

async fn criar_categoria(
    State(state): State<AppState>,
    Json(input): Json<CategoriaInput>,
) -> Result<(StatusCode, Json<CategoriaPessoa>), AppError> {
    validar_categoria(&input)?;
    let item = sqlx::query_as::<_, CategoriaPessoa>(
        "INSERT INTO categoria_pessoa (nome_categoria, cor_hex) VALUES (?, ?) \
         RETURNING id, nome_categoria, cor_hex",
    )
    .bind(input.nome_categoria.trim())
    .bind(input.cor_hex.to_ascii_uppercase())
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn atualizar_categoria(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<CategoriaInput>,
) -> Result<Json<CategoriaPessoa>, AppError> {
    validar_categoria(&input)?;
    let item = sqlx::query_as::<_, CategoriaPessoa>(
        "UPDATE categoria_pessoa SET nome_categoria = ?, cor_hex = ? WHERE id = ? \
         RETURNING id, nome_categoria, cor_hex",
    )
    .bind(input.nome_categoria.trim())
    .bind(input.cor_hex.to_ascii_uppercase())
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("categoria"))?;
    Ok(Json(item))
}

async fn excluir_categoria(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM categoria_pessoa WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("categoria"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_categoria(state: &AppState, id: i64) -> Result<CategoriaPessoa, AppError> {
    sqlx::query_as::<_, CategoriaPessoa>(
        "SELECT id, nome_categoria, cor_hex FROM categoria_pessoa WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("categoria"))
}

async fn listar_tipos(
    State(state): State<AppState>,
) -> Result<Json<Vec<TipoMeioContato>>, AppError> {
    let itens = sqlx::query_as::<_, TipoMeioContato>(
        "SELECT id, nome_tipo FROM tipo_meio_contato ORDER BY nome_tipo",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(itens))
}

async fn obter_tipo_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<TipoMeioContato>, AppError> {
    let item = sqlx::query_as::<_, TipoMeioContato>(
        "SELECT id, nome_tipo FROM tipo_meio_contato WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("tipo de contato"))?;
    Ok(Json(item))
}

async fn criar_tipo_contato(
    State(state): State<AppState>,
    Json(input): Json<TipoMeioContatoInput>,
) -> Result<(StatusCode, Json<TipoMeioContato>), AppError> {
    validar_texto(&input.nome_tipo, "nome_tipo")?;
    let item = sqlx::query_as::<_, TipoMeioContato>(
        "INSERT INTO tipo_meio_contato (nome_tipo) VALUES (?) RETURNING id, nome_tipo",
    )
    .bind(input.nome_tipo.trim())
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn atualizar_tipo_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<TipoMeioContatoInput>,
) -> Result<Json<TipoMeioContato>, AppError> {
    validar_texto(&input.nome_tipo, "nome_tipo")?;
    let item = sqlx::query_as::<_, TipoMeioContato>(
        "UPDATE tipo_meio_contato SET nome_tipo = ? WHERE id = ? RETURNING id, nome_tipo",
    )
    .bind(input.nome_tipo.trim())
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("tipo de contato"))?;
    Ok(Json(item))
}

async fn excluir_tipo_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM tipo_meio_contato WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("tipo de contato"));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn validar_categoria(input: &CategoriaInput) -> Result<(), AppError> {
    validar_texto(&input.nome_categoria, "nome_categoria")?;
    let cor = input.cor_hex.as_bytes();
    if cor.len() != 7 || cor.first() != Some(&b'#') || !cor[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err(AppError::BadRequest(
            "cor_hex deve usar o formato #RRGGBB".to_owned(),
        ));
    }
    Ok(())
}

fn validar_texto(valor: &str, campo: &str) -> Result<(), AppError> {
    if valor.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{campo} é obrigatório")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validar_categoria;
    use crate::models::CategoriaInput;

    #[test]
    fn valida_cor_hexadecimal() {
        assert!(
            validar_categoria(&CategoriaInput {
                nome_categoria: "Amigo".into(),
                cor_hex: "#12aBcF".into(),
            })
            .is_ok()
        );
        assert!(
            validar_categoria(&CategoriaInput {
                nome_categoria: "Amigo".into(),
                cor_hex: "vermelho".into(),
            })
            .is_err()
        );
    }
}
