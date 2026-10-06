use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};
use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};

pub fn rotas() -> Router<AppState> {
    Router::new().route("/", get(buscar))
}

#[derive(Serialize)]
pub struct Pagina<T> {
    pub itens: Vec<T>,
    pub total: i64,
    pub pagina: i64,
    pub por_pagina: i64,
    pub total_paginas: i64,
}
impl<T> Pagina<T> {
    pub fn nova(itens: Vec<T>, total: i64, pagina: i64, por_pagina: i64) -> Self {
        Self {
            itens,
            total,
            pagina,
            por_pagina,
            total_paginas: (total + por_pagina - 1) / por_pagina,
        }
    }
}
pub fn paginacao(pagina: Option<i64>, tamanho: Option<i64>) -> Result<(i64, i64), AppError> {
    let pagina = pagina.unwrap_or(1);
    let tamanho = tamanho.unwrap_or(30);
    if !(1..=1_000_000).contains(&pagina) || !(1..=100).contains(&tamanho) {
        return Err(AppError::BadRequest(
            "pagina deve ser positiva e por_pagina deve estar entre 1 e 100".into(),
        ));
    }
    Ok((pagina, tamanho))
}
pub fn termo_fts(busca: &str) -> Result<Option<String>, AppError> {
    if busca.chars().count() > 200 {
        return Err(AppError::BadRequest(
            "a busca aceita até 200 caracteres".into(),
        ));
    }
    let termos: Vec<_> = busca
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .take(12)
        .map(|s| format!("\"{s}\"*"))
        .collect();
    Ok((!termos.is_empty()).then(|| termos.join(" AND ")))
}
#[derive(Deserialize)]
struct Filtro {
    busca: String,
    tipo: Option<String>,
    pagina: Option<i64>,
    por_pagina: Option<i64>,
}
#[derive(Serialize, sqlx::FromRow)]
struct Resultado {
    tipo: String,
    recurso_id: i64,
    titulo: String,
    resumo: String,
    url: String,
}
async fn buscar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(filtro): Query<Filtro>,
) -> Result<Json<Pagina<Resultado>>, AppError> {
    let (pagina, tamanho) = paginacao(filtro.pagina, filtro.por_pagina)?;
    if filtro
        .tipo
        .as_deref()
        .is_some_and(|t| !matches!(t, "pessoa" | "tarefa" | "vinculo" | "anexo"))
    {
        return Err(AppError::BadRequest("tipo de resultado inválido".into()));
    }
    let Some(termo) = termo_fts(&filtro.busca)? else {
        return Ok(Json(Pagina::nova(vec![], 0, pagina, tamanho)));
    };
    let mut count = QueryBuilder::<Sqlite>::new("SELECT COUNT(*)");
    predicados(
        &mut count,
        &termo,
        sessao.usuario.id,
        filtro.tipo.as_deref(),
    );
    let total: i64 = count.build_query_scalar().fetch_one(&state.pool).await?;
    let select = "SELECT d.tipo,d.recurso_id,d.titulo,snippet(busca_fts,1,'','','…',32) AS resumo, CASE d.tipo WHEN 'pessoa' THEN '/pessoas/'||d.recurso_id WHEN 'tarefa' THEN '/calendario?tarefa='||d.recurso_id WHEN 'vinculo' THEN '/grafo?vinculo='||d.recurso_id WHEN 'anexo_dossie' THEN '/api/dossie/anexos/'||d.recurso_id||'/stream' WHEN 'anexo_vinculo' THEN '/api/vinculos/anexos/'||d.recurso_id||'/stream' ELSE '/api/calendario/anexos/'||d.recurso_id||'/stream' END AS url";
    let mut items = QueryBuilder::<Sqlite>::new(select);
    predicados(
        &mut items,
        &termo,
        sessao.usuario.id,
        filtro.tipo.as_deref(),
    );
    items
        .push(" ORDER BY bm25(busca_fts),d.id LIMIT ")
        .push_bind(tamanho)
        .push(" OFFSET ")
        .push_bind((pagina - 1) * tamanho);
    let itens = items.build_query_as().fetch_all(&state.pool).await?;
    Ok(Json(Pagina::nova(itens, total, pagina, tamanho)))
}
fn predicados(q: &mut QueryBuilder<Sqlite>, termo: &str, usuario_id: i64, tipo: Option<&str>) {
    q.push(" FROM busca_fts JOIN busca_visivel d ON d.id=busca_fts.rowid WHERE busca_fts MATCH ")
        .push_bind(termo.to_owned())
        .push(" AND (d.usuario_id IS NULL OR d.usuario_id=")
        .push_bind(usuario_id)
        .push(")");
    if let Some(tipo) = tipo {
        if tipo == "anexo" {
            q.push(" AND d.tipo LIKE 'anexo_%'");
        } else {
            q.push(" AND d.tipo=").push_bind(tipo.to_owned());
        }
    }
}
