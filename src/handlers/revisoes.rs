use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/{tipo}/{id}", get(listar))
        .route("/{tipo}/{id}/{revisao}/restaurar", post(restaurar))
}
fn tabela(tipo: &str) -> Result<(&'static str, &'static str), AppError> {
    match tipo {
        "pessoa" => Ok(("pessoa", "excluida_em IS NULL")),
        "vinculo" => Ok(("pessoa_vinculo", "excluido_em IS NULL")),
        "tarefa" => Ok(("tarefa_calendario", "1=1")),
        _ => Err(AppError::BadRequest("tipo de registro inválido".into())),
    }
}
pub async fn versao(
    tx: &mut Transaction<'_, Sqlite>,
    tipo: &str,
    id: i64,
    usuario: i64,
) -> Result<i64, AppError> {
    let (table, active) = tabela(tipo)?;
    let query = format!(
        "SELECT versao FROM {table} WHERE id=? AND {active}{}",
        if tipo == "tarefa" {
            " AND usuario_id=?"
        } else {
            ""
        }
    );
    let mut q = sqlx::query_scalar(&query).bind(id);
    if tipo == "tarefa" {
        q = q.bind(usuario)
    }
    q.fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::nao_encontrado("registro ativo"))
}
pub async fn verificar(
    tx: &mut Transaction<'_, Sqlite>,
    tipo: &str,
    id: i64,
    usuario: i64,
    esperada: Option<i64>,
) -> Result<i64, AppError> {
    let atual = versao(tx, tipo, id, usuario).await?;
    if esperada.is_some_and(|v| v != atual) {
        return Err(AppError::Conflict("Este registro foi alterado desde que você abriu a edição. Seus dados não foram salvos. Recarregue o registro e compare as alterações antes de tentar novamente.".into()));
    }
    Ok(atual)
}
pub fn esperada(headers: &HeaderMap) -> Result<Option<i64>, AppError> {
    headers
        .get("if-match")
        .map(|v| {
            v.to_str()
                .ok()
                .and_then(|s| s.trim_matches('"').parse::<i64>().ok())
                .filter(|v| *v > 0)
                .ok_or_else(|| {
                    AppError::BadRequest("If-Match deve conter uma versão positiva".into())
                })
        })
        .transpose()
}
pub async fn capturar(
    tx: &mut Transaction<'_, Sqlite>,
    tipo: &str,
    id: i64,
) -> Result<String, AppError> {
    let query = match tipo {
        "pessoa" => {
            "SELECT json_object('nome',nome,'categoria_nome',(SELECT nome_categoria FROM categoria_pessoa WHERE id=pessoa.categoria_id),'tem_foto',foto_principal IS NOT NULL,'descricao',descricao,'categoria_id',categoria_id,'pessoa_juridica',pessoa_juridica,'classificacao_risco',classificacao_risco,'toxicidade',toxicidade,'risco_justificativa',risco_justificativa,'risco_revisado_em',risco_revisado_em,'contatos',json((SELECT json_group_array(json_object('tipo_contato_id',tipo_contato_id,'valor',valor)) FROM contato WHERE pessoa_id=pessoa.id))) FROM pessoa WHERE id=?"
        }
        "vinculo" => {
            "SELECT json_object('pessoa_origem_id',pessoa_origem_id,'pessoa_destino_id',pessoa_destino_id,'tipo_vinculo',tipo_vinculo,'descricao',descricao) FROM pessoa_vinculo WHERE id=?"
        }
        "tarefa" => {
            "SELECT json_object('titulo',titulo,'descricao',descricao,'inicio_em',inicio_em,'fim_em',fim_em,'dia_inteiro',dia_inteiro,'status',status,'prioridade',prioridade,'cor_hex',cor_hex,'lembrete_minutos',lembrete_minutos,'pessoas',json((SELECT json_group_array(pessoa_id) FROM tarefa_calendario_pessoa WHERE tarefa_id=tarefa_calendario.id))) FROM tarefa_calendario WHERE id=?"
        }
        _ => return Err(AppError::BadRequest("tipo inválido".into())),
    };
    Ok(sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(&mut **tx)
        .await?)
}
pub async fn registrar(
    tx: &mut Transaction<'_, Sqlite>,
    tipo: &str,
    id: i64,
    sessao: &SessaoAutenticada,
    versao: i64,
    snapshot: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO revisao_edicao(tipo,recurso_id,usuario_id,autor_login,versao_anterior,anterior_json) VALUES(?,?,?,?,?,?)").bind(tipo).bind(id).bind(sessao.usuario.id).bind(&sessao.usuario.login).bind(versao).bind(snapshot).execute(&mut **tx).await?;
    Ok(())
}
pub async fn iniciar(
    tx: &mut Transaction<'_, Sqlite>,
    tipo: &str,
    id: i64,
    sessao: &SessaoAutenticada,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let version = verificar(tx, tipo, id, sessao.usuario.id, esperada(headers)?).await?;
    let snapshot = capturar(tx, tipo, id).await?;
    registrar(tx, tipo, id, sessao, version, &snapshot).await
}
#[derive(Serialize, sqlx::FromRow)]
struct Revisao {
    id: i64,
    autor_login: String,
    versao_anterior: i64,
    anterior_json: String,
    criado_em: String,
}
async fn listar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path((tipo, id)): Path<(String, i64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut tx = state.pool.begin().await?;
    let version = versao(&mut tx, &tipo, id, sessao.usuario.id).await?;
    let items:Vec<Revisao>=sqlx::query_as("SELECT id,autor_login,versao_anterior,anterior_json,criado_em FROM revisao_edicao WHERE tipo=? AND recurso_id=? ORDER BY id DESC LIMIT 100").bind(tipo).bind(id).fetch_all(&mut *tx).await?;
    Ok(Json(serde_json::json!({"versao":version,"itens":items})))
}
#[derive(Deserialize)]
struct RestaurarInput {
    versao: i64,
}
async fn restaurar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path((tipo, id, revisao)): Path<(String, i64, i64)>,
    Json(input): Json<RestaurarInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let version = verificar(&mut tx, &tipo, id, sessao.usuario.id, Some(input.versao)).await?;
    if tipo == "pessoa" {
        let limit: i64 =
            sqlx::query_scalar("SELECT mesclagem_revisao_limite FROM pessoa WHERE id=?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if revisao <= limit {
            return Err(AppError::Conflict(
                "Esta revisão é anterior à mesclagem e está disponível apenas para consulta".into(),
            ));
        }
    }
    let snapshot: String = sqlx::query_scalar(
        "SELECT anterior_json FROM revisao_edicao WHERE id=? AND tipo=? AND recurso_id=?",
    )
    .bind(revisao)
    .bind(&tipo)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("revisão"))?;
    let atual = capturar(&mut tx, &tipo, id).await?;
    registrar(&mut tx, &tipo, id, &sessao, version, &atual).await?;
    aplicar(&mut tx, &tipo, id, &snapshot, &sessao).await?;
    let version = versao(&mut tx, &tipo, id, sessao.usuario.id).await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"versao":version})))
}
async fn aplicar(
    tx: &mut Transaction<'_, Sqlite>,
    tipo: &str,
    id: i64,
    snapshot: &str,
    sessao: &SessaoAutenticada,
) -> Result<(), AppError> {
    let fields: &[&str] = match tipo {
        "pessoa" => &[
            "nome",
            "descricao",
            "categoria_id",
            "pessoa_juridica",
            "classificacao_risco",
            "toxicidade",
            "risco_justificativa",
            "risco_revisado_em",
        ],
        "vinculo" => &[
            "pessoa_origem_id",
            "pessoa_destino_id",
            "tipo_vinculo",
            "descricao",
        ],
        "tarefa" => &[
            "titulo",
            "descricao",
            "inicio_em",
            "fim_em",
            "dia_inteiro",
            "status",
            "prioridade",
            "cor_hex",
            "lembrete_minutos",
        ],
        _ => return Err(AppError::BadRequest("tipo inválido".into())),
    };
    let (table, _) = tabela(tipo)?;
    // Reject restored links to people that have been removed or merged.
    let data: serde_json::Value =
        serde_json::from_str(snapshot).map_err(|e| AppError::interno(e.to_string()))?;
    let people: Vec<i64> = if tipo == "vinculo" {
        vec![
            data["pessoa_origem_id"].as_i64().unwrap_or(0),
            data["pessoa_destino_id"].as_i64().unwrap_or(0),
        ]
    } else if tipo == "tarefa" {
        data["pessoas"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_i64())
            .collect()
    } else {
        vec![]
    };
    for person in people {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pessoa WHERE id=? AND excluida_em IS NULL)",
        )
        .bind(person)
        .fetch_one(&mut **tx)
        .await?;
        if !active {
            return Err(AppError::Conflict("A revisão referencia uma pessoa removida ou mesclada. Compare os dados e edite o registro atual.".into()));
        }
    }
    let risco_anterior = if tipo == "pessoa" {
        Some(super::hp_psicossocial::carregar_registro(tx, id).await?)
    } else {
        None
    };
    let assignments = fields
        .iter()
        .map(|f| format!("{f}=json_extract(?,'$.{f}')"))
        .collect::<Vec<_>>()
        .join(",");
    let extra = if tipo == "tarefa" {
        ",lembrete_dispensado_em=NULL,lembrete_adiado_ate=NULL,data_atualizacao=CURRENT_TIMESTAMP"
    } else {
        ""
    };
    let query = format!("UPDATE {table} SET {assignments}{extra} WHERE id=?");
    let mut q = sqlx::query(&query);
    for _ in fields {
        q = q.bind(snapshot)
    }
    q.bind(id).execute(&mut **tx).await?;
    if tipo == "pessoa" {
        sqlx::query("DELETE FROM contato WHERE pessoa_id=?")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO contato(pessoa_id,tipo_contato_id,valor) SELECT ?,json_extract(value,'$.tipo_contato_id'),json_extract(value,'$.valor') FROM json_each(?,'$.contatos')").bind(id).bind(snapshot).execute(&mut **tx).await?;
        let novo = super::hp_psicossocial::carregar_registro(tx, id).await?;
        sqlx::query("INSERT INTO pessoa_risco_historico(pessoa_id,autor_id,autor_login,anterior_json,novo_json) VALUES(?,?,?,?,?)").bind(id).bind(sessao.usuario.id).bind(&sessao.usuario.login).bind(serde_json::to_string(&risco_anterior).map_err(|e|AppError::interno(e.to_string()))?).bind(serde_json::to_string(&novo).map_err(|e|AppError::interno(e.to_string()))?).execute(&mut **tx).await?;
    } else if tipo == "tarefa" {
        sqlx::query("DELETE FROM tarefa_calendario_pessoa WHERE tarefa_id=?")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO tarefa_calendario_pessoa(tarefa_id,pessoa_id) SELECT ?,value FROM json_each(?,'$.pessoas')").bind(id).bind(snapshot).execute(&mut **tx).await?;
    }
    Ok(())
}
