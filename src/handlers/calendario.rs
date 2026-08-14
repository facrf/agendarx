use std::collections::BTreeSet;

use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};
use chrono::{DateTime, Datelike, Duration, SecondsFormat, TimeZone, Timelike, Utc};
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    middleware::auth::SessaoAutenticada,
    models::{
        AnexoTarefaCalendario, AnexoTarefaResumo, ArmazenamentoTarefasResponse, CalendarioFiltro,
        HistoricoTarefaResponse, PessoaTarefaResumo, TarefaCalendarioDataInput,
        TarefaCalendarioInput, TarefaCalendarioResponse, TarefaCalendarioRow,
        TarefaCalendarioStatusInput,
    },
};

const MAX_PESSOAS_POR_TAREFA: usize = 50;
const MAX_ANEXOS_POR_TAREFA: i64 = 30;
const MAX_DIAS_RECORRENCIA: i64 = 366;
const MAX_OCORRENCIAS: usize = 370;

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/tarefas", get(listar_tarefas).post(criar_tarefa))
        .route(
            "/tarefas/{id}",
            get(obter_tarefa)
                .put(atualizar_tarefa)
                .delete(excluir_tarefa),
        )
        .route(
            "/tarefas/{id}/data",
            axum::routing::patch(atualizar_data_tarefa),
        )
        .route(
            "/tarefas/{id}/status",
            axum::routing::patch(atualizar_status_tarefa),
        )
        .route("/tarefas/{id}/historico", get(listar_historico))
        .route(
            "/pessoas/{pessoa_id}/tarefas",
            get(listar_tarefas_da_pessoa),
        )
        .route(
            "/tarefas/{tarefa_id}/anexos",
            get(listar_anexos).post(enviar_anexo),
        )
        .route("/anexos/{id}", get(obter_anexo).delete(excluir_anexo))
        .route("/anexos/{id}/stream", get(stream_anexo))
        .route("/anexos/{id}/download", get(download_anexo))
        .route("/anexos/{id}/thumbnail", get(obter_miniatura))
        .route("/lembretes", get(listar_lembretes))
        .route(
            "/lembretes/{id}/dispensar",
            axum::routing::patch(dispensar_lembrete),
        )
        .route("/armazenamento", get(obter_armazenamento))
}

async fn listar_tarefas_da_pessoa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<TarefaCalendarioResponse>>, AppError> {
    let pessoa_existe: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ?)")
            .bind(pessoa_id)
            .fetch_one(&state.pool)
            .await?;
    if !pessoa_existe {
        return Err(AppError::nao_encontrado("pessoa"));
    }

    let rows = sqlx::query_as::<_, TarefaCalendarioRow>(
        "SELECT t.id, t.usuario_id, t.titulo, t.descricao, t.inicio_em, t.fim_em, \
                t.dia_inteiro, t.status, t.prioridade, t.cor_hex, t.serie_id, \
                t.recorrencia, t.recorrencia_fim_em, t.lembrete_minutos, \
                t.lembrete_dispensado_em, t.data_criacao, t.data_atualizacao \
         FROM tarefa_calendario t \
         JOIN tarefa_calendario_pessoa tp ON tp.tarefa_id = t.id \
         WHERE tp.pessoa_id = ? AND t.usuario_id = ? \
         ORDER BY CASE WHEN t.status = 'CONCLUIDA' THEN 1 ELSE 0 END, \
                  CASE WHEN t.status <> 'CONCLUIDA' THEN t.inicio_em END, \
                  CASE WHEN t.status = 'CONCLUIDA' THEN t.inicio_em END DESC, t.id",
    )
    .bind(pessoa_id)
    .bind(sessao.usuario.id)
    .fetch_all(&state.pool)
    .await?;

    let mut tarefas = Vec::with_capacity(rows.len());
    for row in rows {
        tarefas.push(montar_resposta(&state, row).await?);
    }
    Ok(Json(tarefas))
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
                        status, prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, \
                        lembrete_minutos, lembrete_dispensado_em, data_criacao, data_atualizacao \
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
                        status, prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, \
                        lembrete_minutos, lembrete_dispensado_em, data_criacao, data_atualizacao \
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
                        status, prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, \
                        lembrete_minutos, lembrete_dispensado_em, data_criacao, data_atualizacao \
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
                        status, prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, \
                        lembrete_minutos, lembrete_dispensado_em, data_criacao, data_atualizacao \
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
    let inicios = gerar_inicios_recorrentes(&dados)?;
    let serie_id = (inicios.len() > 1).then(|| Uuid::new_v4().to_string());
    let inicio_original = parse_data(&dados.inicio_em, "inicio_em")?;
    let duracao = dados
        .fim_em
        .as_deref()
        .map(|fim| parse_data(fim, "fim_em").map(|fim| fim - inicio_original))
        .transpose()?;
    let mut tx = state.pool.begin().await?;
    validar_pessoas(&mut tx, &dados.pessoas_ids).await?;

    let mut primeiro_id = None;
    for (indice, inicio) in inicios.into_iter().enumerate() {
        let inicio_em = formatar_data(inicio);
        let fim_em = duracao.map(|duracao| formatar_data(inicio + duracao));
        let status = if indice == 0 {
            &dados.status
        } else {
            "PENDENTE"
        };
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO tarefa_calendario \
                (usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, status, \
                 prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, lembrete_minutos) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(sessao.usuario.id)
        .bind(&dados.titulo)
        .bind(&dados.descricao)
        .bind(inicio_em)
        .bind(fim_em)
        .bind(dados.dia_inteiro)
        .bind(status)
        .bind(&dados.prioridade)
        .bind(&dados.cor_hex)
        .bind(&serie_id)
        .bind(&dados.recorrencia)
        .bind(&dados.recorrencia_fim_em)
        .bind(dados.lembrete_minutos)
        .fetch_one(&mut *tx)
        .await?;
        vincular_pessoas(&mut tx, id, &dados.pessoas_ids).await?;
        registrar_historico_tx(
            &mut tx,
            id,
            "CRIADA",
            if serie_id.is_some() {
                "Ocorrência recorrente criada"
            } else {
                "Tarefa criada"
            },
        )
        .await?;
        primeiro_id.get_or_insert(id);
    }
    tx.commit().await?;

    let id = primeiro_id.ok_or_else(|| AppError::interno("nenhuma ocorrência foi criada"))?;
    let tarefa = buscar_tarefa(&state, sessao.usuario.id, id).await?;
    Ok((StatusCode::CREATED, Json(tarefa)))
}

async fn atualizar_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(mut input): Json<TarefaCalendarioInput>,
) -> Result<Json<TarefaCalendarioResponse>, AppError> {
    // A recorrência é materializada na criação. Depois disso, cada item é
    // independente e uma edição nunca deve recriar ou alterar toda a série.
    input.recorrencia = "NENHUMA".to_owned();
    input.recorrencia_fim_em = None;
    let dados = validar_tarefa(input)?;
    let mut tx = state.pool.begin().await?;
    validar_pessoas(&mut tx, &dados.pessoas_ids).await?;
    let status_anterior = sqlx::query_scalar::<_, String>(
        "SELECT status FROM tarefa_calendario WHERE id = ? AND usuario_id = ?",
    )
    .bind(id)
    .bind(sessao.usuario.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("tarefa"))?;

    let resultado = sqlx::query(
        "UPDATE tarefa_calendario SET titulo = ?, descricao = ?, inicio_em = ?, fim_em = ?, \
                dia_inteiro = ?, status = ?, prioridade = ?, cor_hex = ?, \
                lembrete_minutos = ?, \
                lembrete_dispensado_em = NULL, \
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
    .bind(dados.lembrete_minutos)
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
    if status_anterior != dados.status {
        registrar_historico_tx(
            &mut tx,
            id,
            "STATUS_ALTERADO",
            &format!(
                "Status alterado de {} para {}",
                rotulo_status(&status_anterior),
                rotulo_status(&dados.status)
            ),
        )
        .await?;
    } else {
        registrar_historico_tx(&mut tx, id, "ATUALIZADA", "Dados da tarefa atualizados").await?;
    }
    tx.commit().await?;

    Ok(Json(buscar_tarefa(&state, sessao.usuario.id, id).await?))
}

async fn atualizar_data_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(input): Json<TarefaCalendarioDataInput>,
) -> Result<Json<TarefaCalendarioResponse>, AppError> {
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

    let inicio_novo = formatar_data(inicio);
    let fim_novo = fim.map(formatar_data);
    let mut tx = state.pool.begin().await?;
    let inicio_anterior = sqlx::query_scalar::<_, String>(
        "SELECT inicio_em FROM tarefa_calendario WHERE id = ? AND usuario_id = ?",
    )
    .bind(id)
    .bind(sessao.usuario.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("tarefa"))?;
    let resultado = sqlx::query(
        "UPDATE tarefa_calendario SET inicio_em = ?, fim_em = ?, \
                lembrete_dispensado_em = NULL, \
                data_atualizacao = CURRENT_TIMESTAMP \
         WHERE id = ? AND usuario_id = ?",
    )
    .bind(&inicio_novo)
    .bind(&fim_novo)
    .bind(id)
    .bind(sessao.usuario.id)
    .execute(&mut *tx)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("tarefa"));
    }
    registrar_historico_tx(
        &mut tx,
        id,
        "MOVIDA",
        &format!("Data alterada de {inicio_anterior} para {inicio_novo}"),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(buscar_tarefa(&state, sessao.usuario.id, id).await?))
}

async fn atualizar_status_tarefa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    Json(input): Json<TarefaCalendarioStatusInput>,
) -> Result<Json<TarefaCalendarioResponse>, AppError> {
    let status = validar_status(&input.status)?;
    let mut tx = state.pool.begin().await?;
    let status_anterior = sqlx::query_scalar::<_, String>(
        "SELECT status FROM tarefa_calendario WHERE id = ? AND usuario_id = ?",
    )
    .bind(id)
    .bind(sessao.usuario.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("tarefa"))?;
    if status_anterior != status {
        sqlx::query(
            "UPDATE tarefa_calendario SET status = ?, data_atualizacao = CURRENT_TIMESTAMP \
             WHERE id = ? AND usuario_id = ?",
        )
        .bind(&status)
        .bind(id)
        .bind(sessao.usuario.id)
        .execute(&mut *tx)
        .await?;
        registrar_historico_tx(
            &mut tx,
            id,
            "STATUS_ALTERADO",
            &format!(
                "Status alterado de {} para {}",
                rotulo_status(&status_anterior),
                rotulo_status(&status)
            ),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(buscar_tarefa(&state, sessao.usuario.id, id).await?))
}

async fn listar_historico(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<HistoricoTarefaResponse>>, AppError> {
    garantir_tarefa(&state, sessao.usuario.id, id).await?;
    let itens = sqlx::query_as::<_, HistoricoTarefaResponse>(
        "SELECT id, tarefa_id, tipo, descricao, \
                strftime('%Y-%m-%dT%H:%M:%fZ', data_evento) AS data_evento \
         FROM historico_tarefa_calendario WHERE tarefa_id = ? \
         ORDER BY data_evento DESC, id DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(itens))
}

async fn listar_lembretes(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<Vec<TarefaCalendarioResponse>>, AppError> {
    let agora = Utc::now();
    let limite_inferior = formatar_data(agora - Duration::days(30));
    let limite_superior = formatar_data(agora + Duration::days(366));
    let rows = sqlx::query_as::<_, TarefaCalendarioRow>(
        "SELECT id, usuario_id, titulo, descricao, inicio_em, fim_em, dia_inteiro, \
                status, prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, \
                lembrete_minutos, lembrete_dispensado_em, data_criacao, data_atualizacao \
         FROM tarefa_calendario \
         WHERE usuario_id = ? AND status <> 'CONCLUIDA' \
               AND lembrete_minutos IS NOT NULL AND lembrete_dispensado_em IS NULL \
               AND inicio_em BETWEEN ? AND ? \
         ORDER BY inicio_em, id LIMIT 200",
    )
    .bind(sessao.usuario.id)
    .bind(limite_inferior)
    .bind(limite_superior)
    .fetch_all(&state.pool)
    .await?;

    let mut lembretes = Vec::new();
    for row in rows {
        let minutos = row.lembrete_minutos.unwrap_or_default();
        let inicio = parse_data(&row.inicio_em, "inicio_em")?;
        if inicio - Duration::minutes(minutos) <= agora {
            lembretes.push(montar_resposta(&state, row).await?);
            if lembretes.len() >= 20 {
                break;
            }
        }
    }
    Ok(Json(lembretes))
}

async fn dispensar_lembrete(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query(
        "UPDATE tarefa_calendario SET lembrete_dispensado_em = ? \
         WHERE id = ? AND usuario_id = ? AND lembrete_minutos IS NOT NULL",
    )
    .bind(formatar_data(Utc::now()))
    .bind(id)
    .bind(sessao.usuario.id)
    .execute(&state.pool)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("lembrete"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn obter_armazenamento(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<ArmazenamentoTarefasResponse>, AppError> {
    let (usado_bytes, anexos_total) = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(a.tamanho_bytes), 0), COUNT(a.id) \
         FROM anexo_tarefa_calendario a \
         JOIN tarefa_calendario t ON t.id = a.tarefa_id \
         WHERE t.usuario_id = ?",
    )
    .bind(sessao.usuario.id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(ArmazenamentoTarefasResponse {
        usado_bytes,
        limite_usuario_bytes: state.config.task_storage_quota_bytes,
        limite_tarefa_bytes: state.config.task_storage_per_task_bytes,
        max_arquivo_bytes: state.config.max_upload_bytes as i64,
        anexos_total,
    }))
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
                status, prioridade, cor_hex, serie_id, recorrencia, recorrencia_fim_em, \
                lembrete_minutos, lembrete_dispensado_em, data_criacao, data_atualizacao \
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
        "SELECT p.id, p.nome, c.cor_hex, (p.foto_principal IS NOT NULL) AS tem_foto, p.pessoa_juridica \
         FROM tarefa_calendario_pessoa tp \
         JOIN pessoa p ON p.id = tp.pessoa_id \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE tp.tarefa_id = ? ORDER BY p.nome COLLATE NOCASE",
    )
    .bind(row.id)
    .fetch_all(&state.pool)
    .await?;
    let anexos = buscar_resumos_anexos(state, row.id).await?;

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
        serie_id: row.serie_id.clone(),
        recorrencia: row.recorrencia,
        recorrencia_fim_em: row.recorrencia_fim_em,
        total_ocorrencias: if let Some(serie_id) = &row.serie_id {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM tarefa_calendario WHERE serie_id = ? AND usuario_id = ?",
            )
            .bind(serie_id)
            .bind(row.usuario_id)
            .fetch_one(&state.pool)
            .await?
        } else {
            1
        },
        lembrete_minutos: row.lembrete_minutos,
        pessoas,
        anexos,
        data_criacao: row.data_criacao,
        data_atualizacao: row.data_atualizacao,
    })
}

#[derive(sqlx::FromRow)]
struct AnexoTarefaLinha {
    id: i64,
    tarefa_id: i64,
    nome_arquivo: String,
    mime_type: String,
    tamanho_bytes: i64,
    data_upload: String,
}

impl From<AnexoTarefaLinha> for AnexoTarefaResumo {
    fn from(linha: AnexoTarefaLinha) -> Self {
        let base = format!("/api/calendario/anexos/{}", linha.id);
        let url_thumbnail = super::miniaturas::mime_suportado(&linha.mime_type)
            .then(|| format!("{base}/thumbnail"));
        Self {
            id: linha.id,
            tarefa_id: linha.tarefa_id,
            nome_arquivo: linha.nome_arquivo,
            mime_type: linha.mime_type,
            tamanho_bytes: linha.tamanho_bytes,
            data_upload: linha.data_upload,
            url_stream: format!("{base}/stream"),
            url_download: format!("{base}/download"),
            url_thumbnail,
        }
    }
}

async fn buscar_resumos_anexos(
    state: &AppState,
    tarefa_id: i64,
) -> Result<Vec<AnexoTarefaResumo>, AppError> {
    let linhas = sqlx::query_as::<_, AnexoTarefaLinha>(
        "SELECT id, tarefa_id, nome_arquivo, mime_type, tamanho_bytes, data_upload \
         FROM anexo_tarefa_calendario WHERE tarefa_id = ? \
         ORDER BY data_upload DESC, id DESC",
    )
    .bind(tarefa_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(linhas.into_iter().map(Into::into).collect())
}

async fn listar_anexos(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(tarefa_id): Path<i64>,
) -> Result<Json<Vec<AnexoTarefaResumo>>, AppError> {
    garantir_tarefa(&state, sessao.usuario.id, tarefa_id).await?;
    Ok(Json(buscar_resumos_anexos(&state, tarefa_id).await?))
}

async fn obter_anexo(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<Json<AnexoTarefaResumo>, AppError> {
    let linha = buscar_linha_anexo(&state, sessao.usuario.id, id).await?;
    Ok(Json(linha.into()))
}

async fn enviar_anexo(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(tarefa_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<AnexoTarefaResumo>), AppError> {
    garantir_tarefa(&state, sessao.usuario.id, tarefa_id).await?;
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM anexo_tarefa_calendario WHERE tarefa_id = ?")
            .bind(tarefa_id)
            .fetch_one(&state.pool)
            .await?;
    if total >= MAX_ANEXOS_POR_TAREFA {
        return Err(AppError::BadRequest(format!(
            "cada tarefa aceita no máximo {MAX_ANEXOS_POR_TAREFA} anexos"
        )));
    }

    let mut arquivo = None;
    while let Some(campo) = multipart
        .next_field()
        .await
        .map_err(|erro| AppError::BadRequest(format!("multipart inválido: {erro}")))?
    {
        if campo.name() != Some("arquivo") {
            continue;
        }
        let nome = campo.file_name().unwrap_or("arquivo.bin").to_owned();
        let mime = campo.content_type().map(str::to_owned);
        let conteudo = campo
            .bytes()
            .await
            .map_err(|erro| AppError::BadRequest(format!("falha no upload: {erro}")))?;
        arquivo = Some((nome, mime, conteudo));
        break;
    }

    let (nome, mime_informado, conteudo) = arquivo.ok_or_else(|| {
        AppError::BadRequest("envie o arquivo no campo multipart 'arquivo'".to_owned())
    })?;
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("o arquivo está vazio".to_owned()));
    }
    if conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }
    let nome = super::dossie::normalizar_nome_arquivo(&nome)?;
    let mime_type = infer::get(&conteudo)
        .map(|tipo| tipo.mime_type().to_owned())
        .or_else(|| mime_informado.filter(|mime| mime.parse::<mime::Mime>().is_ok()))
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let miniatura = if super::miniaturas::mime_suportado(&mime_type) {
        super::miniaturas::gerar(conteudo.clone()).await?
    } else {
        None
    };
    let tamanho = conteudo.len() as i64;

    let mut tx = state.pool.begin().await?;
    let usado_tarefa: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(tamanho_bytes), 0) FROM anexo_tarefa_calendario WHERE tarefa_id = ?",
    )
    .bind(tarefa_id)
    .fetch_one(&mut *tx)
    .await?;
    if usado_tarefa + tamanho > state.config.task_storage_per_task_bytes {
        return Err(AppError::BadRequest(
            "o anexo excede o espaço disponível nesta tarefa".to_owned(),
        ));
    }
    let usado_usuario: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(a.tamanho_bytes), 0) \
         FROM anexo_tarefa_calendario a \
         JOIN tarefa_calendario t ON t.id = a.tarefa_id \
         WHERE t.usuario_id = ?",
    )
    .bind(sessao.usuario.id)
    .fetch_one(&mut *tx)
    .await?;
    if usado_usuario + tamanho > state.config.task_storage_quota_bytes {
        return Err(AppError::BadRequest(
            "o anexo excede a cota de armazenamento das tarefas".to_owned(),
        ));
    }
    let linha = sqlx::query_as::<_, AnexoTarefaLinha>(
        "INSERT INTO anexo_tarefa_calendario \
            (tarefa_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes) \
         VALUES (?, ?, ?, ?, ?) \
         RETURNING id, tarefa_id, nome_arquivo, mime_type, tamanho_bytes, data_upload",
    )
    .bind(tarefa_id)
    .bind(nome)
    .bind(mime_type)
    .bind(conteudo.as_ref())
    .bind(tamanho)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(miniatura) = miniatura {
        sqlx::query(
            "INSERT INTO miniatura_anexo_tarefa_calendario \
                (anexo_id, conteudo_webp, largura, altura, tamanho_bytes) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(linha.id)
        .bind(&miniatura.conteudo)
        .bind(i64::from(miniatura.largura))
        .bind(i64::from(miniatura.altura))
        .bind(miniatura.conteudo.len() as i64)
        .execute(&mut *tx)
        .await?;
    }
    registrar_historico_tx(
        &mut tx,
        tarefa_id,
        "ANEXO_ADICIONADO",
        &format!("Anexo adicionado: {}", linha.nome_arquivo),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(linha.into())))
}

async fn stream_anexo(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    servir_anexo(&state, sessao.usuario.id, id, headers, false).await
}

async fn download_anexo(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    servir_anexo(&state, sessao.usuario.id, id, headers, true).await
}

async fn servir_anexo(
    state: &AppState,
    usuario_id: i64,
    id: i64,
    headers: HeaderMap,
    download: bool,
) -> Result<Response, AppError> {
    let anexo = buscar_conteudo_anexo(state, usuario_id, id).await?;
    Ok(super::dossie::servir_blob(
        anexo.conteudo_blob,
        &anexo.mime_type,
        &anexo.nome_arquivo,
        download,
        headers
            .get(header::RANGE)
            .and_then(|valor| valor.to_str().ok()),
    ))
}

async fn obter_miniatura(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    garantir_anexo(&state, sessao.usuario.id, id).await?;
    if let Some(conteudo) = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT conteudo_webp FROM miniatura_anexo_tarefa_calendario WHERE anexo_id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    {
        return Ok(super::miniaturas::responder(conteudo));
    }

    let anexo = buscar_conteudo_anexo(&state, sessao.usuario.id, id).await?;
    if !super::miniaturas::mime_suportado(&anexo.mime_type) {
        return Err(AppError::nao_encontrado("miniatura"));
    }
    let miniatura = super::miniaturas::gerar(Bytes::from(anexo.conteudo_blob))
        .await?
        .ok_or_else(|| AppError::nao_encontrado("miniatura"))?;
    sqlx::query(
        "INSERT INTO miniatura_anexo_tarefa_calendario \
            (anexo_id, conteudo_webp, largura, altura, tamanho_bytes) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(anexo_id) DO UPDATE SET conteudo_webp = excluded.conteudo_webp, \
            largura = excluded.largura, altura = excluded.altura, \
            tamanho_bytes = excluded.tamanho_bytes, data_geracao = CURRENT_TIMESTAMP",
    )
    .bind(id)
    .bind(&miniatura.conteudo)
    .bind(i64::from(miniatura.largura))
    .bind(i64::from(miniatura.altura))
    .bind(miniatura.conteudo.len() as i64)
    .execute(&state.pool)
    .await?;
    Ok(super::miniaturas::responder(miniatura.conteudo))
}

async fn excluir_anexo(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let linha = buscar_linha_anexo(&state, sessao.usuario.id, id).await?;
    let mut tx = state.pool.begin().await?;
    let resultado = sqlx::query(
        "DELETE FROM anexo_tarefa_calendario WHERE id = ? AND EXISTS (\
            SELECT 1 FROM tarefa_calendario t \
            WHERE t.id = anexo_tarefa_calendario.tarefa_id AND t.usuario_id = ?\
        )",
    )
    .bind(id)
    .bind(sessao.usuario.id)
    .execute(&mut *tx)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("anexo"));
    }
    registrar_historico_tx(
        &mut tx,
        linha.tarefa_id,
        "ANEXO_EXCLUIDO",
        &format!("Anexo excluído: {}", linha.nome_arquivo),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn garantir_tarefa(state: &AppState, usuario_id: i64, id: i64) -> Result<(), AppError> {
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tarefa_calendario WHERE id = ? AND usuario_id = ?)",
    )
    .bind(id)
    .bind(usuario_id)
    .fetch_one(&state.pool)
    .await?;
    if !existe {
        return Err(AppError::nao_encontrado("tarefa"));
    }
    Ok(())
}

async fn garantir_anexo(state: &AppState, usuario_id: i64, id: i64) -> Result<(), AppError> {
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM anexo_tarefa_calendario a \
         JOIN tarefa_calendario t ON t.id = a.tarefa_id \
         WHERE a.id = ? AND t.usuario_id = ?)",
    )
    .bind(id)
    .bind(usuario_id)
    .fetch_one(&state.pool)
    .await?;
    if !existe {
        return Err(AppError::nao_encontrado("anexo"));
    }
    Ok(())
}

async fn buscar_linha_anexo(
    state: &AppState,
    usuario_id: i64,
    id: i64,
) -> Result<AnexoTarefaLinha, AppError> {
    sqlx::query_as::<_, AnexoTarefaLinha>(
        "SELECT a.id, a.tarefa_id, a.nome_arquivo, a.mime_type, a.tamanho_bytes, a.data_upload \
         FROM anexo_tarefa_calendario a \
         JOIN tarefa_calendario t ON t.id = a.tarefa_id \
         WHERE a.id = ? AND t.usuario_id = ?",
    )
    .bind(id)
    .bind(usuario_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo"))
}

async fn buscar_conteudo_anexo(
    state: &AppState,
    usuario_id: i64,
    id: i64,
) -> Result<AnexoTarefaCalendario, AppError> {
    sqlx::query_as::<_, AnexoTarefaCalendario>(
        "SELECT a.id, a.tarefa_id, a.nome_arquivo, a.mime_type, a.conteudo_blob, \
                a.tamanho_bytes, a.data_upload \
         FROM anexo_tarefa_calendario a \
         JOIN tarefa_calendario t ON t.id = a.tarefa_id \
         WHERE a.id = ? AND t.usuario_id = ?",
    )
    .bind(id)
    .bind(usuario_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo"))
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
    recorrencia: String,
    recorrencia_fim_em: Option<String>,
    lembrete_minutos: Option<i64>,
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

    let status = validar_status(&input.status)?;
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

    let recorrencia = input.recorrencia.trim().to_ascii_uppercase();
    if !matches!(
        recorrencia.as_str(),
        "NENHUMA" | "DIARIA" | "SEMANAL" | "MENSAL"
    ) {
        return Err(AppError::BadRequest("recorrência inválida".to_owned()));
    }
    let recorrencia_fim = input
        .recorrencia_fim_em
        .filter(|valor| !valor.trim().is_empty())
        .map(|valor| parse_data(&valor, "recorrencia_fim_em"))
        .transpose()?;
    let recorrencia_fim = if recorrencia == "NENHUMA" {
        None
    } else {
        let fim_recorrencia = recorrencia_fim.ok_or_else(|| {
            AppError::BadRequest("informe a data final da recorrência".to_owned())
        })?;
        if fim_recorrencia <= inicio {
            return Err(AppError::BadRequest(
                "a recorrência deve terminar depois do início da tarefa".to_owned(),
            ));
        }
        if fim_recorrencia - inicio > Duration::days(MAX_DIAS_RECORRENCIA) {
            return Err(AppError::BadRequest(format!(
                "a recorrência pode abranger no máximo {MAX_DIAS_RECORRENCIA} dias"
            )));
        }
        Some(fim_recorrencia)
    };
    if input
        .lembrete_minutos
        .is_some_and(|minutos| !(0..=525_600).contains(&minutos))
    {
        return Err(AppError::BadRequest(
            "o lembrete deve estar entre o horário da tarefa e 365 dias antes".to_owned(),
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
        recorrencia,
        recorrencia_fim_em: recorrencia_fim.map(formatar_data),
        lembrete_minutos: input.lembrete_minutos,
    })
}

fn validar_status(valor: &str) -> Result<String, AppError> {
    let status = valor.trim().to_ascii_uppercase();
    if !matches!(status.as_str(), "PENDENTE" | "EM_ANDAMENTO" | "CONCLUIDA") {
        return Err(AppError::BadRequest("status inválido".to_owned()));
    }
    Ok(status)
}

fn gerar_inicios_recorrentes(dados: &TarefaValidada) -> Result<Vec<DateTime<Utc>>, AppError> {
    let inicio = parse_data(&dados.inicio_em, "inicio_em")?;
    if dados.recorrencia == "NENHUMA" {
        return Ok(vec![inicio]);
    }
    let limite = dados
        .recorrencia_fim_em
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("informe o fim da recorrência".to_owned()))
        .and_then(|valor| parse_data(valor, "recorrencia_fim_em"))?;
    let mut ocorrencias = Vec::new();
    let mut atual = inicio;
    let dia_base = inicio.day();
    while atual <= limite {
        ocorrencias.push(atual);
        if ocorrencias.len() > MAX_OCORRENCIAS {
            return Err(AppError::BadRequest(
                "a recorrência gera ocorrências demais".to_owned(),
            ));
        }
        atual = match dados.recorrencia.as_str() {
            "DIARIA" => atual + Duration::days(1),
            "SEMANAL" => atual + Duration::weeks(1),
            "MENSAL" => adicionar_um_mes(atual, dia_base)?,
            _ => break,
        };
    }
    Ok(ocorrencias)
}

fn adicionar_um_mes(data: DateTime<Utc>, dia_base: u32) -> Result<DateTime<Utc>, AppError> {
    let (ano, mes) = if data.month() == 12 {
        (data.year() + 1, 1)
    } else {
        (data.year(), data.month() + 1)
    };
    let ultimo_dia = (28..=31)
        .rev()
        .find(|dia| {
            Utc.with_ymd_and_hms(ano, mes, *dia, 0, 0, 0)
                .single()
                .is_some()
        })
        .ok_or_else(|| AppError::interno("falha ao calcular recorrência mensal"))?;
    Utc.with_ymd_and_hms(
        ano,
        mes,
        dia_base.min(ultimo_dia),
        data.hour(),
        data.minute(),
        data.second(),
    )
    .single()
    .and_then(|valor| valor.with_nanosecond(data.nanosecond()))
    .ok_or_else(|| AppError::interno("falha ao calcular recorrência mensal"))
}

fn rotulo_status(status: &str) -> &'static str {
    match status {
        "CONCLUIDA" => "Concluída",
        "EM_ANDAMENTO" => "Em andamento",
        _ => "Pendente",
    }
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

async fn registrar_historico_tx(
    tx: &mut Transaction<'_, Sqlite>,
    tarefa_id: i64,
    tipo: &str,
    descricao: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO historico_tarefa_calendario (tarefa_id, tipo, descricao) VALUES (?, ?, ?)",
    )
    .bind(tarefa_id)
    .bind(tipo)
    .bind(descricao)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cor_hex_valida, gerar_inicios_recorrentes, validar_tarefa};
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
            recorrencia: "NENHUMA".into(),
            recorrencia_fim_em: None,
            lembrete_minutos: None,
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

    #[test]
    fn recorrencia_mensal_preserva_o_dia_base_apos_fevereiro() {
        let mut tarefa = tarefa_valida();
        tarefa.inicio_em = "2027-01-31T12:30:00Z".into();
        tarefa.fim_em = None;
        tarefa.recorrencia = "MENSAL".into();
        tarefa.recorrencia_fim_em = Some("2027-04-30T23:59:59Z".into());
        let tarefa = validar_tarefa(tarefa).expect("recorrência válida");

        let ocorrencias = gerar_inicios_recorrentes(&tarefa).expect("ocorrências");
        let datas = ocorrencias
            .iter()
            .map(|data| data.format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            datas,
            ["2027-01-31", "2027-02-28", "2027-03-31", "2027-04-30"]
        );
    }

    #[test]
    fn rejeita_recorrencia_maior_que_um_ano() {
        let mut tarefa = tarefa_valida();
        tarefa.recorrencia = "DIARIA".into();
        tarefa.recorrencia_fim_em = Some("2027-08-20T18:00:00Z".into());
        assert!(validar_tarefa(tarefa).is_err());
    }
}
