use std::collections::BTreeMap;

use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use sqlx::SqliteConnection;

use crate::{
    AppState,
    domain::hp_psicossocial::{
        ArestaPsicossocial, ConfigHpPsicossocial, IndicadoresPsicossociais, NoPsicossocial,
        ParametrosHp, calcular_hp_psicossocial, validar_perfil,
    },
    error::AppError,
    middleware::auth::SessaoAutenticada,
};

pub async fn carregar_configuracao(
    conn: &mut SqliteConnection,
) -> Result<ConfigHpPsicossocial, AppError> {
    let (versao, json): (i64, String) = sqlx::query_as(
        "SELECT versao, parametros_json FROM hp_psicossocial_configuracao WHERE id = 1",
    )
    .fetch_one(&mut *conn)
    .await?;
    let mut parametros: ParametrosHp = serde_json::from_str(&json)
        .map_err(|e| AppError::interno(format!("configuração de HP inválida: {e}")))?;
    parametros
        .validar_e_normalizar()
        .map_err(|e| AppError::interno(e.to_string()))?;
    Ok(ConfigHpPsicossocial { versao, parametros })
}

pub async fn calcular_snapshot(
    conn: &mut SqliteConnection,
) -> Result<
    (
        ConfigHpPsicossocial,
        BTreeMap<i64, IndicadoresPsicossociais>,
    ),
    AppError,
> {
    let config = carregar_configuracao(conn).await?;
    let nos = sqlx::query_as::<_, NoPsicossocial>(
        "SELECT id AS pessoa_id, nome, classificacao_risco, toxicidade FROM pessoa WHERE excluida_em IS NULL ORDER BY id",
    ).fetch_all(&mut *conn).await?;
    let arestas = sqlx::query_as::<_, ArestaPsicossocial>(
        "SELECT v.id AS vinculo_id, v.pessoa_origem_id AS origem_id, v.pessoa_destino_id AS destino_id, v.tipo_vinculo \
         FROM pessoa_vinculo v JOIN pessoa a ON a.id = v.pessoa_origem_id JOIN pessoa b ON b.id = v.pessoa_destino_id \
         WHERE a.excluida_em IS NULL AND b.excluida_em IS NULL ORDER BY v.id",
    ).fetch_all(&mut *conn).await?;
    let atualizacoes = calcular_hp_psicossocial(&nos, &arestas, &config)
        .map_err(|e| AppError::interno(e.to_string()))?;
    Ok((
        config,
        atualizacoes
            .into_iter()
            .map(|a| (a.pessoa_id, a.indicadores))
            .collect(),
    ))
}

pub async fn validar_cadastro(
    conn: &mut SqliteConnection,
    id: i64,
    classificacao: Option<&str>,
    toxicidade: Option<f64>,
) -> Result<Option<(String, f64)>, AppError> {
    match (classificacao, toxicidade) {
        (None, None) => Ok(None),
        (Some(classe), Some(t)) => {
            let config = carregar_configuracao(conn).await?;
            validar_perfil(id, classe, t, &config.parametros)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            Ok(Some((classe.into(), t)))
        }
        _ => Err(AppError::BadRequest(
            "envie classificação de risco e toxicidade juntas".into(),
        )),
    }
}

pub async fn obter_configuracao(
    State(state): State<AppState>,
) -> Result<Json<ConfigHpPsicossocial>, AppError> {
    let mut conn = state.pool.acquire().await?;
    Ok(Json(carregar_configuracao(&mut conn).await?))
}

#[derive(Deserialize)]
pub struct ConfiguracaoInput {
    pub versao_esperada: i64,
    #[serde(flatten)]
    pub parametros: ParametrosHp,
}

pub async fn atualizar_configuracao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(mut input): Json<ConfiguracaoInput>,
) -> Result<Json<ConfigHpPsicossocial>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    input
        .parametros
        .validar_e_normalizar()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let atual = carregar_configuracao(&mut tx).await?;
    if atual.versao != input.versao_esperada {
        return Err(AppError::Conflict(
            "configuração alterada por outra sessão; atualize antes de salvar".into(),
        ));
    }
    let nos = sqlx::query_as::<_, NoPsicossocial>(
        "SELECT id AS pessoa_id, nome, classificacao_risco, toxicidade FROM pessoa ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    for no in nos {
        validar_perfil(
            no.pessoa_id,
            &no.classificacao_risco,
            no.toxicidade,
            &input.parametros,
        )
        .map_err(|e| {
            AppError::Conflict(format!("ajuste os perfis antes de alterar os limites: {e}"))
        })?;
    }
    let versao = atual
        .versao
        .checked_add(1)
        .ok_or_else(|| AppError::interno("versão de HP excedida"))?;
    let json =
        serde_json::to_string(&input.parametros).map_err(|e| AppError::interno(e.to_string()))?;
    sqlx::query("UPDATE hp_psicossocial_configuracao SET versao = ?, parametros_json = ?, atualizado_em = CURRENT_TIMESTAMP, atualizado_por = ? WHERE id = 1")
        .bind(versao).bind(json).bind(sessao.usuario.id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(ConfigHpPsicossocial {
        versao,
        parametros: input.parametros,
    }))
}
