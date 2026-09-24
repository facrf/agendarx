use std::collections::BTreeMap;

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
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
    let (nos, arestas) = carregar_rede(conn).await?;
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

async fn carregar_rede(
    conn: &mut SqliteConnection,
) -> Result<(Vec<NoPsicossocial>, Vec<ArestaPsicossocial>), AppError> {
    let nos = sqlx::query_as::<_, NoPsicossocial>(
        "SELECT id AS pessoa_id, nome, classificacao_risco, toxicidade FROM pessoa WHERE excluida_em IS NULL ORDER BY id",
    ).fetch_all(&mut *conn).await?;
    let arestas = sqlx::query_as::<_, ArestaPsicossocial>(
        "SELECT v.id AS vinculo_id, v.pessoa_origem_id AS origem_id, v.pessoa_destino_id AS destino_id, v.tipo_vinculo \
         FROM pessoa_vinculo v JOIN pessoa a ON a.id = v.pessoa_origem_id JOIN pessoa b ON b.id = v.pessoa_destino_id \
         WHERE v.excluido_em IS NULL AND a.excluida_em IS NULL AND b.excluida_em IS NULL ORDER BY v.id",
    ).fetch_all(&mut *conn).await?;
    Ok((nos, arestas))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistroRisco {
    pub classificacao_risco: String,
    pub toxicidade: f64,
    pub justificativa: String,
    pub revisado_em: Option<String>,
}

pub async fn carregar_registro(
    conn: &mut SqliteConnection,
    id: i64,
) -> Result<RegistroRisco, AppError> {
    sqlx::query_as("SELECT classificacao_risco, toxicidade, risco_justificativa AS justificativa, risco_revisado_em AS revisado_em FROM pessoa WHERE id = ? AND excluida_em IS NULL")
        .bind(id).fetch_optional(&mut *conn).await?.ok_or_else(|| AppError::nao_encontrado("pessoa"))
}

pub async fn registrar_revisao(
    conn: &mut SqliteConnection,
    id: i64,
    sessao: &SessaoAutenticada,
    risco: Option<&(String, f64)>,
    justificativa: Option<&str>,
    revisado_em: Option<&str>,
    criando: bool,
) -> Result<(), AppError> {
    let anterior = carregar_registro(conn, id).await?;
    let mut novo = anterior.clone();
    if let Some((classe, t)) = risco {
        novo.classificacao_risco = classe.clone();
        novo.toxicidade = *t;
    }
    if let Some(texto) = justificativa {
        if texto.chars().count() > 5000 {
            return Err(AppError::BadRequest(
                "justificativa deve ter até 5000 caracteres".into(),
            ));
        }
        novo.justificativa = texto.trim().into();
    }
    if let Some(data) = revisado_em {
        novo.revisado_em = if data.is_empty() {
            None
        } else {
            let parsed = chrono::NaiveDate::parse_from_str(data, "%Y-%m-%d").map_err(|_| {
                AppError::BadRequest("data de revisão inválida; use AAAA-MM-DD".into())
            })?;
            if parsed.format("%Y-%m-%d").to_string() != data {
                return Err(AppError::BadRequest(
                    "data de revisão inválida; use AAAA-MM-DD".into(),
                ));
            }
            Some(data.into())
        };
    } else if novo != anterior || (criando && (risco.is_some() || justificativa.is_some())) {
        novo.revisado_em = Some(chrono::Utc::now().date_naive().to_string());
    }
    if !criando && novo == anterior {
        return Ok(());
    }
    sqlx::query("UPDATE pessoa SET risco_justificativa = ?, risco_revisado_em = ? WHERE id = ?")
        .bind(&novo.justificativa)
        .bind(&novo.revisado_em)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    let antes = if criando {
        None
    } else {
        Some(serde_json::to_string(&anterior).map_err(|e| AppError::interno(e.to_string()))?)
    };
    let depois = serde_json::to_string(&novo).map_err(|e| AppError::interno(e.to_string()))?;
    sqlx::query("INSERT INTO pessoa_risco_historico (pessoa_id, autor_id, autor_login, anterior_json, novo_json) VALUES (?, ?, ?, ?, ?)")
        .bind(id).bind(sessao.usuario.id).bind(&sessao.usuario.login).bind(antes).bind(depois).execute(&mut *conn).await?;
    Ok(())
}

#[derive(Serialize)]
pub struct HistoricoRisco {
    id: i64,
    autor_login: String,
    registrado_em: String,
    anterior: Option<RegistroRisco>,
    novo: RegistroRisco,
}

pub async fn historico_risco(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<HistoricoRisco>>, AppError> {
    let mut tx = state.pool.begin().await?;
    carregar_registro(&mut tx, id).await?;
    let rows: Vec<(i64, String, String, Option<String>, String)> = sqlx::query_as("SELECT id, autor_login, registrado_em, anterior_json, novo_json FROM pessoa_risco_historico WHERE pessoa_id = ? ORDER BY id DESC LIMIT 100")
        .bind(id).fetch_all(&mut *tx).await?;
    let mut historico = Vec::new();
    for (id, autor_login, registrado_em, anterior, novo) in rows {
        historico.push(HistoricoRisco {
            id,
            autor_login,
            registrado_em,
            anterior: anterior
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|e| AppError::interno(e.to_string()))?,
            novo: serde_json::from_str(&novo).map_err(|e| AppError::interno(e.to_string()))?,
        });
    }
    tx.commit().await?;
    Ok(Json(historico))
}

#[derive(Deserialize)]
pub struct PreviaRiscoInput {
    pessoa_id: Option<i64>,
    nome: Option<String>,
    classificacao_risco: String,
    toxicidade: f64,
}

#[derive(Serialize)]
pub struct AlteracaoPrevista {
    pessoa_id: i64,
    nome: String,
    hp_antes: Option<f64>,
    hp_depois: f64,
    aura_antes: Option<String>,
    aura_depois: String,
}

#[derive(Serialize)]
pub struct PreviaRisco {
    pessoa: IndicadoresPsicossociais,
    alteracoes: Vec<AlteracaoPrevista>,
    versao_configuracao: i64,
}

pub async fn prever_risco(
    State(state): State<AppState>,
    Json(input): Json<PreviaRiscoInput>,
) -> Result<Json<PreviaRisco>, AppError> {
    let mut tx = state.pool.begin().await?;
    let config = carregar_configuracao(&mut tx).await?;
    validar_perfil(
        input.pessoa_id.unwrap_or(0),
        &input.classificacao_risco,
        input.toxicidade,
        &config.parametros,
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))?;
    let (mut nos, arestas) = carregar_rede(&mut tx).await?;
    let antes: BTreeMap<_, _> = calcular_hp_psicossocial(&nos, &arestas, &config)
        .map_err(|e| AppError::interno(e.to_string()))?
        .into_iter()
        .map(|a| (a.pessoa_id, a.indicadores))
        .collect();
    let id = if let Some(id) = input.pessoa_id {
        let no = nos
            .iter_mut()
            .find(|n| n.pessoa_id == id)
            .ok_or_else(|| AppError::nao_encontrado("pessoa"))?;
        no.classificacao_risco = input.classificacao_risco;
        no.toxicidade = input.toxicidade;
        id
    } else {
        let id = nos
            .iter()
            .map(|n| n.pessoa_id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| AppError::interno("limite de IDs excedido"))?;
        nos.push(NoPsicossocial {
            pessoa_id: id,
            nome: input.nome.unwrap_or_else(|| "Nova pessoa".into()),
            classificacao_risco: input.classificacao_risco,
            toxicidade: input.toxicidade,
        });
        id
    };
    let depois = calcular_hp_psicossocial(&nos, &arestas, &config)
        .map_err(|e| AppError::interno(e.to_string()))?;
    let pessoa = depois
        .iter()
        .find(|a| a.pessoa_id == id)
        .ok_or_else(|| AppError::interno("prévia sem pessoa"))?
        .indicadores
        .clone();
    let mut alteracoes = Vec::new();
    for atualizacao in depois {
        let anterior = antes.get(&atualizacao.pessoa_id);
        let novo = atualizacao.indicadores;
        if anterior.is_some_and(|a| {
            (a.hp - novo.hp).abs() < 1e-9
                && (a.penalidade_propria - novo.penalidade_propria).abs() < 1e-9
                && (a.penalidade_direta - novo.penalidade_direta).abs() < 1e-9
                && (a.penalidade_residual - novo.penalidade_residual).abs() < 1e-9
                && a.aura_nome == novo.aura_nome
                && a.aura_cor_hex == novo.aura_cor_hex
        }) {
            continue;
        }
        let nome = nos
            .iter()
            .find(|n| n.pessoa_id == atualizacao.pessoa_id)
            .ok_or_else(|| AppError::interno("prévia sem nome"))?
            .nome
            .clone();
        alteracoes.push(AlteracaoPrevista {
            pessoa_id: atualizacao.pessoa_id,
            nome,
            hp_antes: anterior.map(|a| a.hp),
            hp_depois: novo.hp,
            aura_antes: anterior.map(|a| a.aura_nome.clone()),
            aura_depois: novo.aura_nome,
        });
    }
    tx.commit().await?;
    Ok(Json(PreviaRisco {
        pessoa,
        alteracoes,
        versao_configuracao: config.versao,
    }))
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
            "envie classificação de risco e intensidade juntas".into(),
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
