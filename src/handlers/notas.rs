use crate::{AppState, error::AppError};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct NotasInput {
    pub notas: String,
}

#[derive(Serialize)]
pub struct NotasResponse {
    pub notas: String,
    pub pessoas_ids: Vec<i64>,
}

pub enum AnexoTipo {
    Dossie,
    Vinculo,
    Tarefa,
}

impl AnexoTipo {
    fn tabela(&self) -> &'static str {
        match self {
            Self::Dossie => "anexo_dossie",
            Self::Vinculo => "anexo_vinculo",
            Self::Tarefa => "anexo_tarefa_calendario",
        }
    }
}

pub async fn obter(
    state: &AppState,
    tipo: AnexoTipo,
    id: i64,
) -> Result<Json<NotasResponse>, AppError> {
    let notas = sqlx::query_scalar::<_, String>(&format!(
        "SELECT notas FROM {} WHERE id = ?",
        tipo.tabela()
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo"))?;
    let query = match tipo {
        AnexoTipo::Dossie => "SELECT pessoa_id FROM anexo_dossie WHERE id = ?",
        AnexoTipo::Vinculo => {
            "SELECT v.pessoa_origem_id FROM pessoa_vinculo v JOIN anexo_vinculo a ON a.vinculo_id = v.id WHERE a.id = ?1 UNION SELECT v.pessoa_destino_id FROM pessoa_vinculo v JOIN anexo_vinculo a ON a.vinculo_id = v.id WHERE a.id = ?1"
        }
        AnexoTipo::Tarefa => {
            "SELECT p.pessoa_id FROM tarefa_calendario_pessoa p JOIN anexo_tarefa_calendario a ON a.tarefa_id = p.tarefa_id WHERE a.id = ?"
        }
    };
    let pessoas_ids = sqlx::query_scalar(query)
        .bind(id)
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(NotasResponse { notas, pessoas_ids }))
}

pub async fn salvar(
    state: &AppState,
    tipo: AnexoTipo,
    id: i64,
    input: NotasInput,
) -> Result<Json<NotasInput>, AppError> {
    if input.notas.chars().count() > 50_000 {
        return Err(AppError::BadRequest(
            "as notas devem ter no máximo 50000 caracteres".to_owned(),
        ));
    }
    let result = sqlx::query(&format!(
        "UPDATE {} SET notas = ? WHERE id = ?",
        tipo.tabela()
    ))
    .bind(&input.notas)
    .bind(id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("anexo"));
    }
    Ok(Json(input))
}
