use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};
use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub fn rotas() -> Router<AppState> {
    Router::new().route("/", get(obter))
}
#[derive(Deserialize)]
struct Periodo {
    inicio: String,
    fim: String,
    dia: Option<String>,
}
#[derive(Serialize, sqlx::FromRow)]
struct TarefaResumo {
    id: i64,
    titulo: String,
    inicio_em: String,
    fim_em: Option<String>,
    dia_inteiro: bool,
    status: String,
    prioridade: String,
}
#[derive(Serialize)]
struct Painel {
    hoje: Vec<TarefaResumo>,
    atrasadas: Vec<TarefaResumo>,
    proximas: Vec<TarefaResumo>,
    total_hoje: i64,
    total_atrasadas: i64,
    total_proximas: i64,
}
async fn obter(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(periodo): Query<Periodo>,
) -> Result<Json<Painel>, AppError> {
    let inicio = DateTime::parse_from_rfc3339(&periodo.inicio)
        .map_err(|_| AppError::BadRequest("inicio inválido".into()))?
        .with_timezone(&Utc);
    let fim = DateTime::parse_from_rfc3339(&periodo.fim)
        .map_err(|_| AppError::BadRequest("fim inválido".into()))?
        .with_timezone(&Utc);
    if fim <= inicio || (fim - inicio).num_hours() > 26 {
        return Err(AppError::BadRequest(
            "informe o início e o fim do dia local".into(),
        ));
    }
    let dia = periodo
        .dia
        .as_deref()
        .map(|value| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest("dia inválido".into()))
        })
        .transpose()?
        .unwrap_or(inicio.date_naive());
    let amanha = dia
        .succ_opt()
        .ok_or_else(|| AppError::BadRequest("dia inválido".into()))?;
    let depois = amanha
        .checked_add_signed(chrono::Duration::days(7))
        .ok_or_else(|| AppError::BadRequest("dia inválido".into()))?;
    let mut grupos = Vec::new();
    let mut totais = Vec::new();
    for condicao in [
        "(dia_inteiro=0 AND julianday(inicio_em)>=julianday(?) AND julianday(inicio_em)<julianday(?)) OR (dia_inteiro=1 AND substr(inicio_em,1,10)>=? AND substr(inicio_em,1,10)<?)",
        "(dia_inteiro=0 AND julianday(COALESCE(fim_em,inicio_em))<julianday(?) AND julianday(inicio_em)<julianday(?)) OR (dia_inteiro=1 AND substr(COALESCE(fim_em,inicio_em),1,10)<? AND substr(inicio_em,1,10)<?)",
        "(dia_inteiro=0 AND julianday(inicio_em)>=julianday(?) AND julianday(inicio_em)<julianday(?)) OR (dia_inteiro=1 AND substr(inicio_em,1,10)>=? AND substr(inicio_em,1,10)<?)",
    ] {
        let grupo = grupos.len();
        let (de, ate) = match grupo {
            0 => (inicio, fim),
            1 => (Utc::now(), inicio),
            _ => (fim, fim + chrono::Duration::days(7)),
        };
        let (dia_de, dia_ate) = match grupo {
            0 => (dia, amanha),
            1 => (dia, dia),
            _ => (amanha, depois),
        };
        let sql = format!(
            "SELECT id,titulo,inicio_em,fim_em,dia_inteiro,status,prioridade FROM tarefa_calendario WHERE usuario_id=? AND status<>'CONCLUIDA' AND ({condicao}) ORDER BY inicio_em,id LIMIT 20"
        );
        let count = format!(
            "SELECT COUNT(*) FROM tarefa_calendario WHERE usuario_id=? AND status<>'CONCLUIDA' AND ({condicao})"
        );
        let de = de.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let ate = ate.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        totais.push(
            sqlx::query_scalar::<_, i64>(&count)
                .bind(sessao.usuario.id)
                .bind(&de)
                .bind(&ate)
                .bind(dia_de.to_string())
                .bind(dia_ate.to_string())
                .fetch_one(&state.pool)
                .await?,
        );
        grupos.push(
            sqlx::query_as::<_, TarefaResumo>(&sql)
                .bind(sessao.usuario.id)
                .bind(&de)
                .bind(&ate)
                .bind(dia_de.to_string())
                .bind(dia_ate.to_string())
                .fetch_all(&state.pool)
                .await?,
        );
    }
    let mut grupos = grupos.into_iter();
    Ok(Json(Painel {
        hoje: grupos.next().unwrap_or_default(),
        atrasadas: grupos.next().unwrap_or_default(),
        proximas: grupos.next().unwrap_or_default(),
        total_hoje: totais[0],
        total_atrasadas: totais[1],
        total_proximas: totais[2],
    }))
}
