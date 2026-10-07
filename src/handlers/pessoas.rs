use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};

use crate::{
    AppState,
    error::AppError,
    middleware::auth::SessaoAutenticada,
    models::{Contato, ContatoInput, PessoaDetalhe, PessoaInput, PessoaResumo, PessoaUpdateInput},
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/", get(listar_pessoas).post(criar_pessoa))
        .route("/paginadas", get(listar_paginadas))
        .route("/risco/previa", post(super::hp_psicossocial::prever_risco))
        .route(
            "/{id}/risco/historico",
            get(super::hp_psicossocial::historico_risco),
        )
        .route(
            "/{id}",
            get(obter_pessoa)
                .put(atualizar_pessoa)
                .delete(excluir_pessoa),
        )
        .route(
            "/{pessoa_id}/contatos",
            get(listar_contatos).post(criar_contato),
        )
        .route(
            "/contatos/{id}",
            get(obter_contato)
                .put(atualizar_contato)
                .delete(excluir_contato),
        )
}

#[derive(serde::Deserialize, Default)]
struct PessoaFiltro {
    busca: Option<String>,
    categoria: Option<String>,
    tipo: Option<String>,
    favoritos: Option<bool>,
    pagina: Option<i64>,
    por_pagina: Option<i64>,
}
const SELECT_PESSOAS: &str = "SELECT p.id,p.versao,p.nome,p.categoria_id,p.descricao,c.nome_categoria,c.cor_hex,p.classificacao_risco,p.toxicidade,(p.foto_principal IS NOT NULL) AS tem_foto,p.pessoa_juridica,p.data_cadastro,COALESCE((SELECT group_concat(e.nome,char(31)) FROM pessoa_etiqueta pe JOIN etiqueta e ON e.id=pe.etiqueta_id WHERE pe.pessoa_id=p.id),'') AS etiquetas,EXISTS(SELECT 1 FROM pessoa_favorita pf WHERE pf.pessoa_id=p.id AND pf.usuario_id=";
fn consulta_pessoas(
    prefixo: &str,
    filtro: &PessoaFiltro,
    usuario: i64,
) -> Result<sqlx::QueryBuilder<'static, sqlx::Sqlite>, AppError> {
    let mut q = sqlx::QueryBuilder::new(prefixo.to_owned());
    q.push_bind(usuario).push(") AS favorito FROM pessoa p LEFT JOIN categoria_pessoa c ON c.id=p.categoria_id WHERE p.excluida_em IS NULL");
    if let Some(busca) = filtro.busca.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Some(termo) = super::busca::termo_fts(busca)? {
            q.push(" AND p.id IN (WITH encontrados AS (SELECT d.tipo,d.recurso_id,d.pessoa_id FROM busca_fts JOIN busca_visivel d ON d.id=busca_fts.rowid WHERE busca_fts MATCH ").push_bind(termo)
             .push(" AND (d.usuario_id IS NULL OR d.usuario_id=").push_bind(usuario).push(")) SELECT pessoa_id FROM encontrados UNION SELECT v.pessoa_destino_id FROM encontrados e JOIN pessoa_vinculo v ON v.id=CASE WHEN e.tipo='vinculo' THEN e.recurso_id ELSE (SELECT vinculo_id FROM anexo_vinculo WHERE id=e.recurso_id AND e.tipo='anexo_vinculo') END UNION SELECT tp.pessoa_id FROM encontrados e JOIN tarefa_calendario_pessoa tp ON tp.tarefa_id=CASE WHEN e.tipo='tarefa' THEN e.recurso_id ELSE (SELECT tarefa_id FROM anexo_tarefa_calendario WHERE id=e.recurso_id AND e.tipo='anexo_tarefa') END)");
        } else {
            q.push(" AND 0");
        }
    }
    if let Some(categoria) = filtro.categoria.as_deref().filter(|s| !s.is_empty()) {
        if categoria == "sem" {
            q.push(" AND p.categoria_id IS NULL");
        } else {
            let id = categoria
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("categoria inválida".into()))?;
            if id <= 0 {
                return Err(AppError::BadRequest("categoria inválida".into()));
            }
            q.push(" AND p.categoria_id=").push_bind(id);
        }
    }
    if let Some(tipo) = filtro.tipo.as_deref().filter(|s| !s.is_empty()) {
        if !matches!(tipo, "fisica" | "juridica") {
            return Err(AppError::BadRequest("tipo de pessoa inválido".into()));
        }
        q.push(" AND p.pessoa_juridica=")
            .push_bind(tipo == "juridica");
    }
    if filtro.favoritos == Some(true) {
        q.push(" AND EXISTS(SELECT 1 FROM pessoa_favorita pf WHERE pf.pessoa_id=p.id AND pf.usuario_id=").push_bind(usuario).push(")");
    }
    Ok(q)
}
async fn listar_pessoas(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(filtro): Query<PessoaFiltro>,
) -> Result<Json<Vec<PessoaResumo>>, AppError> {
    let mut q = consulta_pessoas(SELECT_PESSOAS, &filtro, sessao.usuario.id)?;
    q.push(" ORDER BY favorito DESC,p.nome COLLATE NOCASE,p.id");
    Ok(Json(q.build_query_as().fetch_all(&state.pool).await?))
}
#[derive(serde::Serialize)]
struct PessoasPagina {
    #[serde(flatten)]
    pagina: super::busca::Pagina<PessoaResumo>,
    total_com_foto: i64,
}
async fn listar_paginadas(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(filtro): Query<PessoaFiltro>,
) -> Result<Json<PessoasPagina>, AppError> {
    let (pagina, tamanho) = super::busca::paginacao(filtro.pagina, filtro.por_pagina)?;
    let prefixo = format!("SELECT COUNT(*),COALESCE(SUM(tem_foto),0) FROM ({SELECT_PESSOAS}");
    let mut count = consulta_pessoas(&prefixo, &filtro, sessao.usuario.id)?;
    count.push(")");
    let (total, fotos): (i64, i64) = count.build_query_as().fetch_one(&state.pool).await?;
    let mut q = consulta_pessoas(SELECT_PESSOAS, &filtro, sessao.usuario.id)?;
    q.push(" ORDER BY favorito DESC,p.nome COLLATE NOCASE,p.id LIMIT ")
        .push_bind(tamanho)
        .push(" OFFSET ")
        .push_bind((pagina - 1) * tamanho);
    let itens = q.build_query_as().fetch_all(&state.pool).await?;
    Ok(Json(PessoasPagina {
        pagina: super::busca::Pagina::nova(itens, total, pagina, tamanho),
        total_com_foto: fotos,
    }))
}

async fn obter_pessoa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<Json<PessoaDetalhe>, AppError> {
    Ok(Json(
        buscar_pessoa_detalhe(&state, id, sessao.usuario.id).await?,
    ))
}

async fn criar_pessoa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<PessoaInput>,
) -> Result<(StatusCode, Json<PessoaDetalhe>), AppError> {
    validar_nome(&input.nome)?;
    validar_descricao(input.descricao.as_deref())?;
    for contato in &input.contatos {
        validar_contato(contato)?;
    }

    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let risco = super::hp_psicossocial::validar_cadastro(
        &mut tx,
        0,
        input.classificacao_risco.as_deref(),
        input.toxicidade,
    )
    .await?
    .unwrap_or_else(|| ("NAO_CLASSIFICADO".into(), 0.0));
    let pessoa_id: i64 = sqlx::query_scalar(
        "INSERT INTO pessoa (nome, categoria_id, descricao, pessoa_juridica, classificacao_risco, toxicidade) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(input.nome.trim())
    .bind(input.categoria_id)
    .bind(normalizar_descricao(input.descricao))
    .bind(input.pessoa_juridica)
    .bind(&risco.0)
    .bind(risco.1)
    .fetch_one(&mut *tx)
    .await?;

    super::hp_psicossocial::registrar_revisao(
        &mut tx,
        pessoa_id,
        &sessao,
        input.classificacao_risco.as_ref().map(|_| &risco),
        input.risco_justificativa.as_deref(),
        input.risco_revisado_em.as_deref(),
        true,
    )
    .await?;

    for contato in input.contatos {
        sqlx::query("INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?)")
            .bind(pessoa_id)
            .bind(contato.tipo_contato_id)
            .bind(contato.valor.trim())
            .execute(&mut *tx)
            .await?;
    }
    let pessoa = buscar_pessoa_detalhe_tx(&mut tx, pessoa_id, sessao.usuario.id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(pessoa)))
}

async fn atualizar_pessoa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<PessoaUpdateInput>,
) -> Result<Json<PessoaDetalhe>, AppError> {
    validar_nome(&input.nome)?;
    validar_descricao(input.descricao.as_deref())?;
    if let Some(contatos) = &input.contatos {
        let mut ids = std::collections::HashSet::new();
        for contato in contatos {
            validar_contato(&contato.contato)?;
            if let Some(id) = contato.id
                && !ids.insert(id)
            {
                return Err(AppError::BadRequest("contato repetido".to_owned()));
            }
        }
    }
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    super::revisoes::iniciar(&mut tx, "pessoa", id, &sessao, &headers).await?;
    let risco = super::hp_psicossocial::validar_cadastro(
        &mut tx,
        id,
        input.classificacao_risco.as_deref(),
        input.toxicidade,
    )
    .await?;
    super::hp_psicossocial::registrar_revisao(
        &mut tx,
        id,
        &sessao,
        risco.as_ref(),
        input.risco_justificativa.as_deref(),
        input.risco_revisado_em.as_deref(),
        false,
    )
    .await?;
    let resultado = sqlx::query("UPDATE pessoa SET nome = ?, categoria_id = ?, descricao = ?, pessoa_juridica = ?, classificacao_risco = COALESCE(?, classificacao_risco), toxicidade = COALESCE(?, toxicidade) WHERE id = ? AND excluida_em IS NULL")
            .bind(input.nome.trim())
            .bind(input.categoria_id)
            .bind(normalizar_descricao(input.descricao))
            .bind(input.pessoa_juridica)
            .bind(risco.as_ref().map(|r| &r.0))
            .bind(risco.as_ref().map(|r| r.1))
            .bind(id)
            .execute(&mut *tx)
            .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    if let Some(contatos) = input.contatos {
        let anteriores: Vec<i64> = sqlx::query_scalar("SELECT id FROM contato WHERE pessoa_id = ?")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
        for anterior in anteriores {
            if !contatos.iter().any(|contato| contato.id == Some(anterior)) {
                sqlx::query("DELETE FROM contato WHERE id = ? AND pessoa_id = ?")
                    .bind(anterior)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        for item in contatos {
            if let Some(contato_id) = item.id {
                let atualizado = sqlx::query("UPDATE contato SET tipo_contato_id = ?, valor = ? WHERE id = ? AND pessoa_id = ?")
                    .bind(item.contato.tipo_contato_id).bind(item.contato.valor.trim())
                    .bind(contato_id).bind(id).execute(&mut *tx).await?;
                if atualizado.rows_affected() == 0 {
                    return Err(AppError::BadRequest(
                        "contato não pertence à pessoa".to_owned(),
                    ));
                }
            } else {
                sqlx::query(
                    "INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?)",
                )
                .bind(id)
                .bind(item.contato.tipo_contato_id)
                .bind(item.contato.valor.trim())
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    let pessoa = buscar_pessoa_detalhe_tx(&mut tx, id, sessao.usuario.id).await?;
    tx.commit().await?;
    Ok(Json(pessoa))
}

async fn excluir_pessoa(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query(
        "UPDATE pessoa SET excluida_em = CURRENT_TIMESTAMP WHERE id = ? AND excluida_em IS NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn listar_contatos(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<Contato>>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let contatos = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato \
         WHERE pessoa_id = ? ORDER BY id",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(contatos))
}

async fn obter_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Contato>, AppError> {
    let contato = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    Ok(Json(contato))
}

async fn criar_contato(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    headers: HeaderMap,
    Path(pessoa_id): Path<i64>,
    Json(input): Json<ContatoInput>,
) -> Result<(StatusCode, Json<Contato>), AppError> {
    validar_contato(&input)?;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    super::revisoes::iniciar(&mut tx, "pessoa", pessoa_id, &sessao, &headers).await?;
    let contato = sqlx::query_as::<_, Contato>(
        "INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?) \
         RETURNING id, pessoa_id, tipo_contato_id, valor",
    )
    .bind(pessoa_id)
    .bind(input.tipo_contato_id)
    .bind(input.valor.trim())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(contato)))
}

async fn atualizar_contato(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ContatoInput>,
) -> Result<Json<Contato>, AppError> {
    validar_contato(&input)?;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let pessoa_id: i64 = sqlx::query_scalar("SELECT pessoa_id FROM contato WHERE id=?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    super::revisoes::iniciar(&mut tx, "pessoa", pessoa_id, &sessao, &headers).await?;
    let contato = sqlx::query_as::<_, Contato>(
        "UPDATE contato SET tipo_contato_id = ?, valor = ? WHERE id = ? \
         RETURNING id, pessoa_id, tipo_contato_id, valor",
    )
    .bind(input.tipo_contato_id)
    .bind(input.valor.trim())
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    tx.commit().await?;
    Ok(Json(contato))
}

async fn excluir_contato(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let pessoa_id: i64 = sqlx::query_scalar("SELECT pessoa_id FROM contato WHERE id=?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    super::revisoes::iniciar(&mut tx, "pessoa", pessoa_id, &sessao, &headers).await?;
    let resultado = sqlx::query("DELETE FROM contato WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("contato"));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_pessoa_detalhe(
    state: &AppState,
    id: i64,
    usuario_id: i64,
) -> Result<PessoaDetalhe, AppError> {
    let mut tx = state.pool.begin().await?;
    let detalhe = buscar_pessoa_detalhe_tx(&mut tx, id, usuario_id).await?;
    tx.commit().await?;
    Ok(detalhe)
}
async fn buscar_pessoa_detalhe_tx(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    usuario_id: i64,
) -> Result<PessoaDetalhe, AppError> {
    let pessoa = sqlx::query_as::<_, PessoaResumo>(
        "SELECT p.id, p.versao, p.nome, p.categoria_id, p.descricao, c.nome_categoria, c.cor_hex, p.classificacao_risco, p.toxicidade, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.pessoa_juridica, p.data_cadastro, \
                COALESCE((SELECT group_concat(e.nome, char(31)) FROM pessoa_etiqueta pe JOIN etiqueta e ON e.id=pe.etiqueta_id WHERE pe.pessoa_id=p.id), '') AS etiquetas, \
                EXISTS(SELECT 1 FROM pessoa_favorita pf WHERE pf.pessoa_id=p.id AND pf.usuario_id=?) AS favorito \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE p.id = ? AND p.excluida_em IS NULL",
    )
    .bind(usuario_id)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("pessoa"))?;
    let contatos = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato \
         WHERE pessoa_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let (_, mut indicadores) = super::hp_psicossocial::calcular_snapshot(&mut *conn).await?;
    let psicossocial = indicadores
        .remove(&id)
        .ok_or_else(|| AppError::interno("perfil ausente do snapshot de HP"))?;
    let risco_registro = super::hp_psicossocial::carregar_registro(&mut *conn, id).await?;
    Ok(PessoaDetalhe {
        pessoa,
        contatos,
        psicossocial,
        risco_registro,
    })
}

async fn garantir_pessoa(state: &AppState, id: i64) -> Result<(), AppError> {
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ? AND excluida_em IS NULL)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    if !existe {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(())
}

fn validar_nome(nome: &str) -> Result<(), AppError> {
    if nome.trim().is_empty() {
        return Err(AppError::BadRequest("nome é obrigatório".to_owned()));
    }
    Ok(())
}

fn validar_descricao(descricao: Option<&str>) -> Result<(), AppError> {
    if descricao.is_some_and(|valor| valor.chars().count() > 50_000) {
        return Err(AppError::BadRequest(
            "a descrição deve ter no máximo 50000 caracteres".to_owned(),
        ));
    }
    Ok(())
}

fn normalizar_descricao(descricao: Option<String>) -> Option<String> {
    descricao.and_then(|valor| {
        let valor = valor.trim().to_owned();
        (!valor.is_empty()).then_some(valor)
    })
}

fn validar_contato(input: &ContatoInput) -> Result<(), AppError> {
    if input.tipo_contato_id <= 0 || input.valor.trim().is_empty() {
        return Err(AppError::BadRequest(
            "tipo_contato_id e valor são obrigatórios".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{normalizar_descricao, validar_descricao};

    #[test]
    fn normaliza_descricao_vazia_e_remove_espacos() {
        assert_eq!(normalizar_descricao(Some("   ".to_owned())), None);
        assert_eq!(
            normalizar_descricao(Some("  Perfil detalhado  ".to_owned())),
            Some("Perfil detalhado".to_owned())
        );
    }

    #[test]
    fn limita_descricao_por_caracteres() {
        assert!(validar_descricao(Some(&"á".repeat(50_000))).is_ok());
        assert!(validar_descricao(Some(&"á".repeat(50_001))).is_err());
    }
}
