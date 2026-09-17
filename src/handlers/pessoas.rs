use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
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
}

async fn listar_pessoas(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(filtro): Query<PessoaFiltro>,
) -> Result<Json<Vec<PessoaResumo>>, AppError> {
    let busca = filtro
        .busca
        .map(|v| normalizar_busca(v.trim()))
        .filter(|v| !v.is_empty());
    let conteudo = if busca.is_some() {
        "trim(COALESCE(p.descricao,'') || ' ' || COALESCE((SELECT group_concat(co.valor, ' ') FROM contato co WHERE co.pessoa_id=p.id),'') || ' ' || COALESCE((SELECT group_concat(a.nome_arquivo || ' ' || a.notas || ' ' || CASE WHEN a.mime_type LIKE 'text/%' OR lower(a.nome_arquivo) GLOB '*.md' OR lower(a.nome_arquivo) GLOB '*.txt' THEN CAST(a.conteudo_blob AS TEXT) ELSE '' END, ' ') FROM anexo_dossie a WHERE a.pessoa_id=p.id),'') || ' ' || COALESCE((SELECT group_concat(av.nome_arquivo || ' ' || av.notas || ' ' || CASE WHEN av.mime_type LIKE 'text/%' OR lower(av.nome_arquivo) GLOB '*.md' OR lower(av.nome_arquivo) GLOB '*.txt' THEN CAST(av.conteudo_blob AS TEXT) ELSE '' END, ' ') FROM anexo_vinculo av JOIN pessoa_vinculo pv ON pv.id=av.vinculo_id WHERE pv.pessoa_origem_id=p.id OR pv.pessoa_destino_id=p.id),'') || ' ' || COALESCE((SELECT group_concat(at.nome_arquivo || ' ' || at.notas || ' ' || CASE WHEN at.mime_type LIKE 'text/%' OR lower(at.nome_arquivo) GLOB '*.md' OR lower(at.nome_arquivo) GLOB '*.txt' THEN CAST(at.conteudo_blob AS TEXT) ELSE '' END, ' ') FROM anexo_tarefa_calendario at JOIN tarefa_calendario_pessoa tp ON tp.tarefa_id=at.tarefa_id WHERE tp.pessoa_id=p.id),'') || ' ' || COALESCE((SELECT group_concat(e.nome, ' ') FROM pessoa_etiqueta pe JOIN etiqueta e ON e.id=pe.etiqueta_id WHERE pe.pessoa_id=p.id),''))"
    } else {
        "''"
    };
    let sql = format!(
        "SELECT p.id, p.nome, p.categoria_id, p.descricao, c.nome_categoria, c.cor_hex, p.classificacao_risco, p.toxicidade, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.pessoa_juridica, p.data_cadastro, \
                COALESCE((SELECT group_concat(e.nome, char(31)) FROM pessoa_etiqueta pe JOIN etiqueta e ON e.id=pe.etiqueta_id WHERE pe.pessoa_id=p.id), '') AS etiquetas, \
                {conteudo} AS conteudo_busca, \
                EXISTS(SELECT 1 FROM pessoa_favorita pf WHERE pf.pessoa_id=p.id AND pf.usuario_id=?) AS favorito \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE p.excluida_em IS NULL \
         ORDER BY p.nome COLLATE NOCASE",
    );
    let mut pessoas = sqlx::query_as::<_, PessoaResumo>(&sql)
        .bind(sessao.usuario.id)
        .fetch_all(&state.pool)
        .await?;
    if let Some(busca) = busca {
        pessoas.retain(|pessoa| {
            let texto = format!(
                "{} {} {}",
                pessoa.nome, pessoa.etiquetas, pessoa.conteudo_busca
            );
            normalizar_busca(&texto).contains(&busca)
        });
    }
    Ok(Json(pessoas))
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

    for contato in input.contatos {
        sqlx::query("INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?)")
            .bind(pessoa_id)
            .bind(contato.tipo_contato_id)
            .bind(contato.valor.trim())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    let pessoa = buscar_pessoa_detalhe(&state, pessoa_id, sessao.usuario.id).await?;
    Ok((StatusCode::CREATED, Json(pessoa)))
}

async fn atualizar_pessoa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
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
    let risco = super::hp_psicossocial::validar_cadastro(
        &mut tx,
        id,
        input.classificacao_risco.as_deref(),
        input.toxicidade,
    )
    .await?;
    let resultado =
        sqlx::query("UPDATE pessoa SET nome = ?, categoria_id = ?, descricao = ?, pessoa_juridica = ?, classificacao_risco = COALESCE(?, classificacao_risco), toxicidade = COALESCE(?, toxicidade) WHERE id = ? AND excluida_em IS NULL")
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
    tx.commit().await?;
    Ok(Json(
        buscar_pessoa_detalhe(&state, id, sessao.usuario.id).await?,
    ))
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
    Path(pessoa_id): Path<i64>,
    Json(input): Json<ContatoInput>,
) -> Result<(StatusCode, Json<Contato>), AppError> {
    validar_contato(&input)?;
    let contato = sqlx::query_as::<_, Contato>(
        "INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?) \
         RETURNING id, pessoa_id, tipo_contato_id, valor",
    )
    .bind(pessoa_id)
    .bind(input.tipo_contato_id)
    .bind(input.valor.trim())
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(contato)))
}

async fn atualizar_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<ContatoInput>,
) -> Result<Json<Contato>, AppError> {
    validar_contato(&input)?;
    let contato = sqlx::query_as::<_, Contato>(
        "UPDATE contato SET tipo_contato_id = ?, valor = ? WHERE id = ? \
         RETURNING id, pessoa_id, tipo_contato_id, valor",
    )
    .bind(input.tipo_contato_id)
    .bind(input.valor.trim())
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("contato"))?;
    Ok(Json(contato))
}

async fn excluir_contato(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("DELETE FROM contato WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("contato"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_pessoa_detalhe(
    state: &AppState,
    id: i64,
    usuario_id: i64,
) -> Result<PessoaDetalhe, AppError> {
    let mut tx = state.pool.begin().await?;
    let pessoa = sqlx::query_as::<_, PessoaResumo>(
        "SELECT p.id, p.nome, p.categoria_id, p.descricao, c.nome_categoria, c.cor_hex, p.classificacao_risco, p.toxicidade, \
                (p.foto_principal IS NOT NULL) AS tem_foto, p.pessoa_juridica, p.data_cadastro, \
                COALESCE((SELECT group_concat(e.nome, char(31)) FROM pessoa_etiqueta pe JOIN etiqueta e ON e.id=pe.etiqueta_id WHERE pe.pessoa_id=p.id), '') AS etiquetas, \
                '' AS conteudo_busca, \
                EXISTS(SELECT 1 FROM pessoa_favorita pf WHERE pf.pessoa_id=p.id AND pf.usuario_id=?) AS favorito \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE p.id = ? AND p.excluida_em IS NULL",
    )
    .bind(usuario_id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("pessoa"))?;
    let contatos = sqlx::query_as::<_, Contato>(
        "SELECT id, pessoa_id, tipo_contato_id, valor FROM contato \
         WHERE pessoa_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let (_, mut indicadores) = super::hp_psicossocial::calcular_snapshot(&mut tx).await?;
    let psicossocial = indicadores
        .remove(&id)
        .ok_or_else(|| AppError::interno("perfil ausente do snapshot de HP"))?;
    tx.commit().await?;
    Ok(PessoaDetalhe {
        pessoa,
        contatos,
        psicossocial,
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

fn normalizar_busca(valor: &str) -> String {
    valor
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
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
    use super::{normalizar_busca, normalizar_descricao, validar_descricao};

    #[test]
    fn busca_ignora_acentos_e_caixa() {
        assert_eq!(normalizar_busca("REUNIÃO São"), "reuniao sao");
    }

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
