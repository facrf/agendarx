use std::collections::BTreeSet;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{Sqlite, Transaction};

use crate::{
    AppState,
    error::AppError,
    middleware::auth::SessaoAutenticada,
    models::{
        CalendarioFiltro, PessoaTarefaResumo, TarefaCalendarioInput, TarefaCalendarioResponse,
        TarefaCalendarioRow,
    },
};

const MAX_PESSOAS_POR_TAREFA: usize = 50;

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/tarefas", get(listar_tarefas).post(criar_tarefa))
        .route(
            "/tarefas/{id}",
            get(obter_tarefa)
                .put(atualizar_tarefa)
                .delete(excluir_tarefa),
        )
}

async fn listar_tarefas(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(filtro): Query<CalendarioFiltro>,
) -> Result<Json<Vec<TarefaCalendarioResponse>>, AppError> {
    let inicio = normalizar_data_opcional(filtro.inicio.as_deref(), "inicio")?;
    let fim = normalizar_data_opcional(filtro.fim.as_deref(), "fim")?;
    if let (Some(inicio), Some(fim)) = (&inicio, &fim)
        && fim <= inicio
    {
        return Err(AppError::BadRequest(
            "o fim do período deve ser posterior ao início".to_owned(),
        ));
    }

    let rows = match (&inicio, &fim) {
        (Some(inicio), Some(fim)) => {
            sqlx::query_as::<_, TarefaCalendarioRow>(
                "SELECT id, usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, \
                        status, prioridade, cor_hex, data_criacao, data_atualizacao \
                 FROM tarefa_calendario \
                 WHERE usuario_id = ? AND inicio_em < ? \
                       AND COALESCE(fim_em, inicio_em) >= ? \
                 ORDER BY inicio_em, id",
            )
            .bind(sessao.usuario.id)
            .bind(fim)
            .bind(inicio)
            .fetch_all(&state.pool)
            .await?
        }
        (Some(inicio), None) => {
            sqlx::query_as::<_, TarefaCalendarioRow>(
                "SELECT id, usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, \
                        status, prioridade, cor_hex, data_criacao, data_atualizacao \
                 FROM tarefa_calendario \
                 WHERE usuario_id = ? AND COALESCE(fim_em, inicio_em) >= ? \
                 ORDER BY inicio_em, id",
            )
            .bind(sessao.usuario.id)
            .bind(inicio)
            .fetch_all(&state.pool)
            .await?
        }
        (None, Some(fim)) => {
            sqlx::query_as::<_, TarefaCalendarioRow>(
                "SELECT id, usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, \
                        status, prioridade, cor_hex, data_criacao, data_atualizacao \
                 FROM tarefa_calendario \
                 WHERE usuario_id = ? AND inicio_em < ? \
                 ORDER BY inicio_em, id",
            )
            .bind(sessao.usuario.id)
            .bind(fim)
            .fetch_all(&state.pool)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, TarefaCalendarioRow>(
                "SELECT id, usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, \
                        status, prioridade, cor_hex, data_criacao, data_atualizacao \
                 FROM tarefa_calendario WHERE usuario_id = ? ORDER BY inicio_em, id",
            )
            .bind(sessao.usuario.id)
            .fetch_all(&state.pool)
            .await?
        }
    };

    let mut tarefas = Vec::with_capacity(rows.len());
    for row in rows {
        tarefas.push(montar_resposta(&state, row).await?);
    }
    Ok(Json(tarefas))
}

async fn obter_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<Json<TarefaCalendarioResponse>, AppError> {
    Ok(Json(buscar_tarefa(&state, sessao.usuario.id, id).await?))
}

async fn criar_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<TarefaCalendarioInput>,
) -> Result<(StatusCode, Json<TarefaCalendarioResponse>), AppError> {
    let dados = validar_tarefa(input)?;
    let mut tx = state.pool.begin().await?;
    validar_pessoas(&mut tx, &dados.pessoas_ids).await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO tarefa_calendario \
            (usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, status, prioridade, cor_hex) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(sessao.usuario.id)
    .bind(&dados.titulo)
    .bind(&dados.descricao)
    .bind(&dados.inicio_em)
    .bind(&dados.fim_em)
    .bind(dados.dia_inteiro)
    .bind(&dados.status)
    .bind(&dados.prioridade)
    .bind(&dados.cor_hex)
    .fetch_one(&mut *tx)
    .await?;
    vincular_pessoas(&mut tx, id, &dados.pessoas_ids).await?;
    tx.commit().await?;

    let tarefa = buscar_tarefa(&state, sessao.usuario.id, id).await?;
    Ok((StatusCode::CREATED, Json(tarefa)))
}

async fn atualizar_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(input): Json<TarefaCalendarioInput>,
) -> Result<Json<TarefaCalendarioResponse>, AppError> {
    let dados = validar_tarefa(input)?;
    let mut tx = state.pool.begin().await?;
    validar_pessoas(&mut tx, &dados.pessoas_ids).await?;

    let resultado = sqlx::query(
        "UPDATE tarefa_calendario SET titulo = ?, descricao = ?, inicio_em = ?, fim_em = ?, \
                dia_inteiro = ?, status = ?, prioridade = ?, cor_hex = ?, \
                data_atualizacao = CURRENT_TIMESTAMP \
         WHERE id = ? AND usuario_id = ?",
    )
    .bind(&dados.titulo)
    .bind(&dados.descricao)
    .bind(&dados.inicio_em)
    .bind(&dados.fim_em)
    .bind(dados.dia_inteiro)
    .bind(&dados.status)
    .bind(&dados.prioridade)
    .bind(&dados.cor_hex)
    .bind(id)
    .bind(sessao.usuario.id)
    .execute(&mut *tx)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("tarefa"));
    }

    sqlx::query("DELETE FROM tarefa_calendario_pessoa WHERE tarefa_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    vincular_pessoas(&mut tx, id, &dados.pessoas_ids).await?;
    tx.commit().await?;

    Ok(Json(buscar_tarefa(&state, sessao.usuario.id, id).await?))
}

async fn excluir_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM tarefa_calendario WHERE id = ? AND usuario_id = ?")
        .bind(id)
        .bind(sessao.usuario.id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("tarefa"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_tarefa(
    state: &AppState,
    usuario_id: i64,
    id: i64,
) -> Result<TarefaCalendarioResponse, AppError> {
    let row = sqlx::query_as::<_, TarefaCalendarioRow>(
        "SELECT id, usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, \
                status, prioridade, cor_hex, data_criacao, data_atualizacao \
         FROM tarefa_calendario WHERE id = ? AND usuario_id = ?",
    )
    .bind(id)
    .bind(usuario_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("tarefa"))?;
    montar_resposta(state, row).await
}

async fn montar_resposta(
    state: &AppState,
    row: TarefaCalendarioRow,
) -> Result<TarefaCalendarioResponse, AppError> {
    let pessoas = sqlx::query_as::<_, PessoaTarefaResumo>(
        "SELECT p.id, p.nome, c.cor_hex, (p.foto_principal IS NOT NULL) AS tem_foto \
         FROM tarefa_calendario_pessoa tp \
         JOIN pessoa p ON p.id = tp.pessoa_id \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE tp.tarefa_id = ? ORDER BY p.nome COLLATE NOCASE",
    )
    .bind(row.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(TarefaCalendarioResponse {
        id: row.id,
        titulo: row.titulo,
        descricao: row.descricao,
        inicio_em: row.inicio_em,
        fim_em: row.fim_em,
        dia_inteiro: row.dia_inteiro,
        status: row.status,
        prioridade: row.prioridade,
        cor_hex: row.cor_hex,
        pessoas,
        data_criacao: row.data_criacao,
        data_atualizacao: row.data_atualizacao,
    })
}

struct TarefaValidada {
    titulo: String,
    descricao: Option<String>,
    inicio_em: String,
    fim_em: Option<String>,
    dia_inteiro: bool,
    status: String,
    prioridade: String,
    cor_hex: String,
    pessoas_ids: Vec<i64>,
}

fn validar_tarefa(input: TarefaCalendarioInput) -> Result<TarefaValidada, AppError> {
    let titulo = input.titulo.trim().to_owned();
    if !(1..=160).contains(&titulo.chars().count()) {
        return Err(AppError::BadRequest(
            "o título deve possuir entre 1 e 160 caracteres".to_owned(),
        ));
    }

    let descricao = input.descricao.and_then(|valor| {
        let limpa = valor.trim().to_owned();
        (!limpa.is_empty()).then_some(limpa)
    });
    if descricao
        .as_ref()
        .is_some_and(|valor| valor.chars().count() > 5000)
    {
        return Err(AppError::BadRequest(
            "a descrição deve possuir no máximo 5000 caracteres".to_owned(),
        ));
    }

    let inicio = parse_data(&input.inicio_em, "inicio_em")?;
    let fim = input
        .fim_em
        .filter(|valor| !valor.trim().is_empty())
        .map(|valor| parse_data(&valor, "fim_em"))
        .transpose()?;
    if fim.is_some_and(|fim| fim <= inicio) {
        return Err(AppError::BadRequest(
            "fim_em deve ser posterior a inicio_em".to_owned(),
        ));
    }

    let status = input.status.trim().to_ascii_uppercase();
    if !matches!(status.as_str(), "PENDENTE" | "EM_ANDAMENTO" | "CONCLUIDA") {
        return Err(AppError::BadRequest("status inválido".to_owned()));
    }
    let prioridade = input.prioridade.trim().to_ascii_uppercase();
    if !matches!(prioridade.as_str(), "BAIXA" | "NORMAL" | "ALTA") {
        return Err(AppError::BadRequest("prioridade inválida".to_owned()));
    }
    let cor_hex = input.cor_hex.trim().to_ascii_uppercase();
    if !cor_hex_valida(&cor_hex) {
        return Err(AppError::BadRequest(
            "cor_hex deve usar o formato #RRGGBB".to_owned(),
        ));
    }

    let pessoas_ids = input.pessoas_ids.into_iter().collect::<BTreeSet<_>>();
    if pessoas_ids.len() > MAX_PESSOAS_POR_TAREFA || pessoas_ids.iter().any(|id| *id <= 0) {
        return Err(AppError::BadRequest(
            "vincule no máximo 50 pessoas válidas por tarefa".to_owned(),
        ));
    }

    Ok(TarefaValidada {
        titulo,
        descricao,
        inicio_em: formatar_data(inicio),
        fim_em: fim.map(formatar_data),
        dia_inteiro: input.dia_inteiro,
        status,
        prioridade,
        cor_hex,
        pessoas_ids: pessoas_ids.into_iter().collect(),
    })
}

fn parse_data(valor: &str, campo: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(valor.trim())
        .map(|data| data.with_timezone(&Utc))
        .map_err(|_| AppError::BadRequest(format!("{campo} deve ser uma data ISO 8601 válida")))
}

fn normalizar_data_opcional(valor: Option<&str>, campo: &str) -> Result<Option<String>, AppError> {
    valor
        .filter(|valor| !valor.trim().is_empty())
        .map(|valor| parse_data(valor, campo).map(formatar_data))
        .transpose()
}

fn formatar_data(data: DateTime<Utc>) -> String {
    data.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn cor_hex_valida(cor: &str) -> bool {
    let bytes = cor.as_bytes();
    bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit)
}

async fn validar_pessoas(
    tx: &mut Transaction<'_, Sqlite>,
    pessoas_ids: &[i64],
) -> Result<(), AppError> {
    for pessoa_id in pessoas_ids {
        let existe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ?)")
            .bind(pessoa_id)
            .fetch_one(&mut **tx)
            .await?;
        if !existe {
            return Err(AppError::BadRequest(format!(
                "a pessoa de id {pessoa_id} não existe"
            )));
        }
    }
    Ok(())
}

async fn vincular_pessoas(
    tx: &mut Transaction<'_, Sqlite>,
    tarefa_id: i64,
    pessoas_ids: &[i64],
) -> Result<(), AppError> {
    for pessoa_id in pessoas_ids {
        sqlx::query("INSERT INTO tarefa_calendario_pessoa (tarefa_id, pessoa_id) VALUES (?, ?)")
            .bind(tarefa_id)
            .bind(pessoa_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cor_hex_valida, validar_tarefa};
    use crate::models::TarefaCalendarioInput;

    fn tarefa_valida() -> TarefaCalendarioInput {
        TarefaCalendarioInput {
            titulo: "Telefonar para Ana".into(),
            descricao: None,
            inicio_em: "2026-08-12T15:00:00-03:00".into(),
            fim_em: Some("2026-08-12T16:00:00-03:00".into()),
            dia_inteiro: false,
            status: "PENDENTE".into(),
            prioridade: "NORMAL".into(),
            cor_hex: "#13716D".into(),
            pessoas_ids: vec![2, 1, 2],
        }
    }

    #[test]
    fn normaliza_datas_e_remove_pessoas_duplicadas() {
        let tarefa = validar_tarefa(tarefa_valida()).expect("tarefa válida");
        assert_eq!(tarefa.inicio_em, "2026-08-12T18:00:00.000Z");
        assert_eq!(tarefa.pessoas_ids, vec![1, 2]);
    }

    #[test]
    fn rejeita_periodo_invertido_e_cor_invalida() {
        let mut tarefa = tarefa_valida();
        tarefa.fim_em = Some("2026-08-12T14:00:00-03:00".into());
        assert!(validar_tarefa(tarefa).is_err());
        assert!(cor_hex_valida("#12ABef"));
        assert!(!cor_hex_valida("coral"));
    }
}
