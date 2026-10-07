use std::collections::{BTreeMap, BTreeSet};

use axum::{
    Extension, Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use super::{ContatoImportado, ler_importacao, persistir_contatos};
use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Coincidencia {
    pessoa_id: i64,
    nome: String,
}
#[derive(Serialize, Deserialize)]
pub(super) struct Registro {
    indice: usize,
    contato: ContatoImportado,
    coincidencias: Vec<Coincidencia>,
    repetidos_no_arquivo: Vec<usize>,
    acao_sugerida: String,
}
#[derive(Serialize, Deserialize)]
pub(super) struct Previa {
    token: String,
    expira_em: i64,
    registros: Vec<Registro>,
    registros_ignorados: usize,
    avisos: Vec<String>,
}
#[derive(Deserialize)]
pub(super) struct Confirmacao {
    decisoes: Vec<Decisao>,
}
#[derive(Deserialize)]
struct Decisao {
    indice: usize,
    acao: String,
    pessoa_id: Option<i64>,
}

pub(super) fn chave_contato(tipo: &str, valor: &str) -> Option<String> {
    let valor = valor.trim();
    let tipo = tipo.to_lowercase();
    if tipo.replace(['-', ' ', '_'], "").contains("mail") && valor.contains('@') {
        return Some(format!("email:{}", valor.to_lowercase()));
    }
    if ["telefone", "celular", "phone", "mobile", "whatsapp"]
        .iter()
        .any(|t| tipo.contains(t))
    {
        let mut digits: String = valor.chars().filter(char::is_ascii_digit).collect();
        if (digits.len() == 12 || digits.len() == 13) && digits.starts_with("55") {
            digits = digits[2..].into();
        }
        if digits.len() >= 7 {
            return Some(format!("fone:{digits}"));
        }
    }
    None
}
fn chaves(contato: &ContatoImportado) -> BTreeSet<String> {
    contato
        .campos
        .iter()
        .filter_map(|c| chave_contato(&c.tipo, &c.valor))
        .collect()
}
async fn indice(
    conn: &mut SqliteConnection,
) -> Result<BTreeMap<String, Vec<Coincidencia>>, AppError> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as("SELECT p.id,p.nome,t.nome_tipo,c.valor FROM pessoa p JOIN contato c ON c.pessoa_id=p.id JOIN tipo_meio_contato t ON t.id=c.tipo_contato_id WHERE p.excluida_em IS NULL ORDER BY p.id,c.id")
        .fetch_all(conn).await?;
    let mut index: BTreeMap<String, Vec<Coincidencia>> = BTreeMap::new();
    for (pessoa_id, nome, tipo, valor) in rows {
        if let Some(key) = chave_contato(&tipo, &valor) {
            let entries = index.entry(key).or_default();
            if !entries.iter().any(|e| e.pessoa_id == pessoa_id) {
                entries.push(Coincidencia { pessoa_id, nome });
            }
        }
    }
    Ok(index)
}
fn coincidencias(
    contato: &ContatoImportado,
    index: &BTreeMap<String, Vec<Coincidencia>>,
) -> Vec<Coincidencia> {
    let mut matches = BTreeMap::new();
    for key in chaves(contato) {
        if let Some(entries) = index.get(&key) {
            for entry in entries {
                matches.insert(entry.pessoa_id, entry.clone());
            }
        }
    }
    matches.into_values().collect()
}
pub(super) async fn preparar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    multipart: Multipart,
) -> Result<Json<Previa>, AppError> {
    let (pessoas, registros_ignorados, avisos) =
        ler_importacao(State(state.clone()), multipart).await?;
    let mut conn = state.pool.acquire().await?;
    let index = indice(&mut conn).await?;
    let mut keys: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, pessoa) in pessoas.iter().enumerate() {
        for key in chaves(pessoa) {
            keys.entry(key).or_default().push(i);
        }
    }
    let registros = pessoas
        .into_iter()
        .enumerate()
        .map(|(i, contato)| {
            let coincidencias = coincidencias(&contato, &index);
            let repetidos: BTreeSet<usize> = chaves(&contato)
                .iter()
                .flat_map(|k| keys.get(k).into_iter().flatten().copied())
                .filter(|j| *j != i)
                .collect();
            let acao_sugerida = if coincidencias.is_empty() && repetidos.is_empty() {
                "criar"
            } else {
                "ignorar"
            }
            .into();
            Registro {
                indice: i,
                contato,
                coincidencias,
                repetidos_no_arquivo: repetidos.into_iter().collect(),
                acao_sugerida,
            }
        })
        .collect();
    let previa = Previa {
        token: uuid::Uuid::new_v4().to_string(),
        expira_em: Utc::now().timestamp() + 1800,
        registros,
        registros_ignorados,
        avisos,
    };
    // Uma prévia por conta limita dados temporários e substitui arquivos enviados anteriormente.
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("DELETE FROM importacao_previa WHERE usuario_id=? OR expira_em<=?")
        .bind(sessao.usuario.id)
        .bind(Utc::now().timestamp())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO importacao_previa VALUES(?,?,?,?)")
        .bind(&previa.token)
        .bind(sessao.usuario.id)
        .bind(serde_json::to_string(&previa).map_err(|e| AppError::interno(e.to_string()))?)
        .bind(previa.expira_em)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(previa))
}
pub(super) async fn confirmar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(token): Path<String>,
    Json(input): Json<Confirmacao>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let data: String = sqlx::query_scalar(
        "SELECT conteudo FROM importacao_previa WHERE token=? AND usuario_id=? AND expira_em>?",
    )
    .bind(&token)
    .bind(sessao.usuario.id)
    .bind(Utc::now().timestamp())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        AppError::NotFound("prévia inexistente ou expirada; envie o arquivo novamente".into())
    })?;
    let previa: Previa =
        serde_json::from_str(&data).map_err(|e| AppError::interno(e.to_string()))?;
    if input.decisoes.len() != previa.registros.len() {
        return Err(AppError::BadRequest(
            "escolha uma ação para cada contato".into(),
        ));
    }
    let mut decisoes = BTreeMap::new();
    for d in input.decisoes {
        if !["ignorar", "criar", "atualizar"].contains(&d.acao.as_str())
            || decisoes.insert(d.indice, d).is_some()
        {
            return Err(AppError::BadRequest(
                "decisões inválidas ou repetidas".into(),
            ));
        }
    }
    let index = indice(&mut tx).await?;
    let mut pessoas = Vec::new();
    let mut ignorados = previa.registros_ignorados;
    let mut atualizados = 0;
    for registro in previa.registros {
        let d = decisoes
            .remove(&registro.indice)
            .ok_or_else(|| AppError::BadRequest("índice de contato inválido".into()))?;
        if d.acao == "ignorar" {
            ignorados += 1;
            continue;
        }
        let atuais = coincidencias(&registro.contato, &index);
        if atuais != registro.coincidencias {
            return Err(AppError::Conflict(
                "os contatos mudaram após a prévia; envie o arquivo novamente".into(),
            ));
        }
        let destino = if d.acao == "atualizar" {
            let id = d
                .pessoa_id
                .filter(|id| atuais.iter().any(|c| c.pessoa_id == *id))
                .ok_or_else(|| {
                    AppError::BadRequest("selecione uma coincidência válida para atualizar".into())
                })?;
            atualizados += 1;
            Some(id)
        } else {
            None
        };
        pessoas.push((registro.contato, destino));
    }
    if !decisoes.is_empty() {
        return Err(AppError::BadRequest("índice de contato inválido".into()));
    }
    for id in pessoas
        .iter()
        .filter_map(|(_, id)| *id)
        .collect::<std::collections::BTreeSet<_>>()
    {
        let version =
            super::super::revisoes::versao(&mut tx, "pessoa", id, sessao.usuario.id).await?;
        let snapshot = super::super::revisoes::capturar(&mut tx, "pessoa", id).await?;
        super::super::revisoes::registrar(&mut tx, "pessoa", id, &sessao, version, &snapshot)
            .await?;
    }
    let mut avisos = previa.avisos;
    let (total, contatos) = persistir_contatos(&mut tx, pessoas, &mut avisos).await?;
    sqlx::query("DELETE FROM importacao_previa WHERE token=?")
        .bind(token)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(
        serde_json::json!({"pessoas_importadas":total-atualizados,"pessoas_atualizadas":atualizados,"contatos_importados":contatos,"registros_ignorados":ignorados,"avisos":avisos}),
    ))
}
pub(super) async fn descartar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(token): Path<String>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM importacao_previa WHERE token=? AND usuario_id=?")
        .bind(token)
        .bind(sessao.usuario.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
