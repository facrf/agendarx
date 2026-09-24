use std::collections::HashMap;

use axum::{
    Extension, Json, Router,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};
use bytes::Bytes;
use serde::Serialize;

use crate::{
    AppState,
    error::AppError,
    middleware::auth::SessaoAutenticada,
    models::{
        AnexoNomeInput, AnexoVinculo, AnexoVinculoResumo, GrafoContato, GrafoEdge, GrafoNode,
        GrafoResponse, PessoaVinculo, VinculoInput,
    },
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/", get(listar_vinculos).post(criar_vinculo))
        .route("/grafo", get(obter_grafo))
        .route("/lixeira", get(listar_lixeira))
        .route(
            "/lixeira/{id}/restaurar",
            axum::routing::post(restaurar_vinculo),
        )
        .route(
            "/lixeira/{id}",
            axum::routing::delete(excluir_definitivamente),
        )
        .route(
            "/{vinculo_id}/anexos",
            get(listar_anexos).post(enviar_anexo),
        )
        .route(
            "/anexos/{id}",
            get(obter_metadados_anexo)
                .put(atualizar_nome_anexo)
                .delete(excluir_anexo),
        )
        .route("/anexos/{id}/stream", get(stream_anexo))
        .route("/anexos/{id}/notas", get(obter_notas).put(salvar_notas))
        .route("/anexos/{id}/download", get(download_anexo))
        .route("/anexos/{id}/thumbnail", get(obter_miniatura))
        .route(
            "/{id}",
            get(obter_vinculo)
                .put(atualizar_vinculo)
                .delete(excluir_vinculo),
        )
}

async fn obter_notas(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<super::notas::NotasResponse>, AppError> {
    garantir_anexo_ativo(&state, id).await?;
    super::notas::obter(&state, super::notas::AnexoTipo::Vinculo, id).await
}

async fn salvar_notas(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<super::notas::NotasInput>,
) -> Result<Json<super::notas::NotasInput>, AppError> {
    garantir_anexo_ativo(&state, id).await?;
    super::notas::salvar(&state, super::notas::AnexoTipo::Vinculo, id, input).await
}

async fn listar_anexos(
    State(state): State<AppState>,
    Path(vinculo_id): Path<i64>,
) -> Result<Json<Vec<AnexoVinculoResumo>>, AppError> {
    buscar_vinculo(&state, vinculo_id).await?;
    let anexos = sqlx::query_as::<_, AnexoVinculoLinha>(
        "SELECT id, vinculo_id, nome_arquivo, mime_type, tamanho_bytes, data_upload \
         FROM anexo_vinculo WHERE vinculo_id = ? ORDER BY data_upload DESC, id DESC",
    )
    .bind(vinculo_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(anexos.into_iter().map(Into::into).collect()))
}

async fn obter_metadados_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<AnexoVinculoResumo>, AppError> {
    garantir_anexo_ativo(&state, id).await?;
    let anexo = sqlx::query_as::<_, AnexoVinculoLinha>(
        "SELECT id, vinculo_id, nome_arquivo, mime_type, tamanho_bytes, data_upload \
         FROM anexo_vinculo WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo do vínculo"))?;
    Ok(Json(anexo.into()))
}

async fn enviar_anexo(
    State(state): State<AppState>,
    Path(vinculo_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<AnexoVinculoResumo>), AppError> {
    buscar_vinculo(&state, vinculo_id).await?;
    let mut arquivo = None;

    while let Some(campo) = multipart.next_field().await.map_err(AppError::from)? {
        if campo.name() != Some("arquivo") {
            continue;
        }
        let nome_arquivo = campo.file_name().unwrap_or("arquivo.bin").to_owned();
        let mime_informado = campo.content_type().map(str::to_owned);
        let conteudo = campo.bytes().await.map_err(AppError::from)?;
        arquivo = Some((nome_arquivo, mime_informado, conteudo));
        break;
    }

    let (nome_arquivo, mime_informado, conteudo) = arquivo.ok_or_else(|| {
        AppError::BadRequest("envie o arquivo no campo multipart 'arquivo'".to_owned())
    })?;
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("o arquivo está vazio".to_owned()));
    }
    if conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }
    let nome_arquivo = super::dossie::normalizar_nome_arquivo(&nome_arquivo)?;

    let mime_type = infer::get(&conteudo)
        .map(|tipo| tipo.mime_type().to_owned())
        .or_else(|| mime_informado.filter(|mime| mime.parse::<mime::Mime>().is_ok()))
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let miniatura = if super::miniaturas::mime_suportado(&mime_type) {
        super::miniaturas::gerar(conteudo.clone()).await?
    } else {
        None
    };
    let tamanho_bytes = conteudo.len() as i64;
    let mut tx = state.pool.begin().await?;
    let anexo = sqlx::query_as::<_, AnexoVinculoLinha>(
        "INSERT INTO anexo_vinculo \
            (vinculo_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes) \
         VALUES (?, ?, ?, ?, ?) \
         RETURNING id, vinculo_id, nome_arquivo, mime_type, tamanho_bytes, data_upload",
    )
    .bind(vinculo_id)
    .bind(nome_arquivo)
    .bind(mime_type)
    .bind(conteudo.as_ref())
    .bind(tamanho_bytes)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(miniatura) = miniatura {
        sqlx::query(
            "INSERT INTO miniatura_anexo_vinculo \
                (anexo_id, conteudo_webp, largura, altura, tamanho_bytes) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(anexo.id)
        .bind(&miniatura.conteudo)
        .bind(i64::from(miniatura.largura))
        .bind(i64::from(miniatura.altura))
        .bind(miniatura.conteudo.len() as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(anexo.into())))
}

async fn atualizar_nome_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<AnexoNomeInput>,
) -> Result<Json<AnexoVinculoResumo>, AppError> {
    garantir_anexo_ativo(&state, id).await?;
    let nome_arquivo = super::dossie::normalizar_nome_arquivo(&input.nome_arquivo)?;
    let anexo = sqlx::query_as::<_, AnexoVinculoLinha>(
        "UPDATE anexo_vinculo SET nome_arquivo = ? WHERE id = ? \
         RETURNING id, vinculo_id, nome_arquivo, mime_type, tamanho_bytes, data_upload",
    )
    .bind(nome_arquivo)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo do vínculo"))?;
    Ok(Json(anexo.into()))
}

async fn stream_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let anexo = buscar_anexo(&state, id).await?;
    Ok(super::dossie::servir_blob(
        anexo.conteudo_blob,
        &anexo.mime_type,
        &anexo.nome_arquivo,
        false,
        headers.get(header::RANGE).and_then(|v| v.to_str().ok()),
    ))
}

async fn download_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let anexo = buscar_anexo(&state, id).await?;
    Ok(super::dossie::servir_blob(
        anexo.conteudo_blob,
        &anexo.mime_type,
        &anexo.nome_arquivo,
        true,
        headers.get(header::RANGE).and_then(|v| v.to_str().ok()),
    ))
}

async fn obter_miniatura(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    garantir_anexo_ativo(&state, id).await?;
    if let Some(conteudo) = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT conteudo_webp FROM miniatura_anexo_vinculo WHERE anexo_id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    {
        return Ok(super::miniaturas::responder(conteudo));
    }

    let anexo = buscar_anexo(&state, id).await?;
    if !super::miniaturas::mime_suportado(&anexo.mime_type) {
        return Err(AppError::nao_encontrado("miniatura"));
    }
    let miniatura = super::miniaturas::gerar(Bytes::from(anexo.conteudo_blob))
        .await?
        .ok_or_else(|| AppError::nao_encontrado("miniatura"))?;
    sqlx::query(
        "INSERT INTO miniatura_anexo_vinculo \
            (anexo_id, conteudo_webp, largura, altura, tamanho_bytes) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(anexo_id) DO UPDATE SET \
            conteudo_webp = excluded.conteudo_webp, largura = excluded.largura, \
            altura = excluded.altura, tamanho_bytes = excluded.tamanho_bytes, \
            data_geracao = CURRENT_TIMESTAMP",
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
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    garantir_anexo_ativo(&state, id).await?;
    let resultado = sqlx::query("DELETE FROM anexo_vinculo WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("anexo do vínculo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_anexo(state: &AppState, id: i64) -> Result<AnexoVinculo, AppError> {
    garantir_anexo_ativo(state, id).await?;
    sqlx::query_as::<_, AnexoVinculo>(
        "SELECT id, vinculo_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes, data_upload \
         FROM anexo_vinculo WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo do vínculo"))
}

async fn garantir_anexo_ativo(state: &AppState, id: i64) -> Result<(), AppError> {
    let ativo: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM anexo_vinculo a JOIN pessoa_vinculo v ON v.id = a.vinculo_id WHERE a.id = ? AND v.excluido_em IS NULL)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    if ativo {
        Ok(())
    } else {
        Err(AppError::nao_encontrado("anexo do vínculo"))
    }
}

async fn listar_vinculos(
    State(state): State<AppState>,
) -> Result<Json<Vec<PessoaVinculo>>, AppError> {
    let vinculos = sqlx::query_as::<_, PessoaVinculo>(
        "SELECT id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao \
         FROM pessoa_vinculo WHERE excluido_em IS NULL ORDER BY data_criacao DESC, id DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(vinculos))
}

async fn obter_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<PessoaVinculo>, AppError> {
    let vinculo = buscar_vinculo(&state, id).await?;
    Ok(Json(vinculo))
}

async fn criar_vinculo(
    State(state): State<AppState>,
    Json(input): Json<VinculoInput>,
) -> Result<(StatusCode, Json<PessoaVinculo>), AppError> {
    validar_vinculo(&input)?;
    let na_lixeira: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pessoa_vinculo WHERE pessoa_origem_id = ? AND pessoa_destino_id = ? AND tipo_vinculo = ? AND excluido_em IS NOT NULL)",
    )
    .bind(input.pessoa_origem_id)
    .bind(input.pessoa_destino_id)
    .bind(input.tipo_vinculo.trim())
    .fetch_one(&state.pool)
    .await?;
    if na_lixeira {
        return Err(AppError::Conflict(
            "esse vínculo está na lixeira; restaure-o para recuperar também os anexos".to_owned(),
        ));
    }
    let vinculo = sqlx::query_as::<_, PessoaVinculo>(
        "INSERT INTO pessoa_vinculo \
            (pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao) \
         VALUES (?, ?, ?, ?) \
         RETURNING id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao",
    )
    .bind(input.pessoa_origem_id)
    .bind(input.pessoa_destino_id)
    .bind(input.tipo_vinculo.trim())
    .bind(normalizar_descricao(input.descricao))
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(vinculo)))
}

async fn atualizar_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<VinculoInput>,
) -> Result<Json<PessoaVinculo>, AppError> {
    validar_vinculo(&input)?;
    let vinculo = sqlx::query_as::<_, PessoaVinculo>(
        "UPDATE pessoa_vinculo SET \
            pessoa_origem_id = ?, pessoa_destino_id = ?, tipo_vinculo = ?, descricao = ? \
         WHERE id = ? AND excluido_em IS NULL \
         RETURNING id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao",
    )
    .bind(input.pessoa_origem_id)
    .bind(input.pessoa_destino_id)
    .bind(input.tipo_vinculo.trim())
    .bind(normalizar_descricao(input.descricao))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("vínculo"))?;
    Ok(Json(vinculo))
}

async fn excluir_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query("UPDATE pessoa_vinculo SET excluido_em = CURRENT_TIMESTAMP WHERE id = ? AND excluido_em IS NULL")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("vínculo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, sqlx::FromRow)]
struct VinculoLixeira {
    id: i64,
    tipo_vinculo: String,
    pessoa_origem_id: i64,
    pessoa_destino_id: i64,
    origem_nome: String,
    destino_nome: String,
    excluido_em: String,
}

async fn listar_lixeira(
    State(state): State<AppState>,
) -> Result<Json<Vec<VinculoLixeira>>, AppError> {
    let vinculos = sqlx::query_as(
        "SELECT v.id, v.tipo_vinculo, v.pessoa_origem_id, v.pessoa_destino_id, \
                origem.nome AS origem_nome, destino.nome AS destino_nome, v.excluido_em \
         FROM pessoa_vinculo v \
         JOIN pessoa origem ON origem.id = v.pessoa_origem_id \
         JOIN pessoa destino ON destino.id = v.pessoa_destino_id \
         WHERE v.excluido_em IS NOT NULL ORDER BY v.excluido_em DESC, v.id DESC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(vinculos))
}

async fn restaurar_vinculo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query(
        "UPDATE pessoa_vinculo SET excluido_em = NULL WHERE id = ? AND excluido_em IS NOT NULL \
         AND pessoa_origem_id IN (SELECT id FROM pessoa WHERE excluida_em IS NULL) \
         AND pessoa_destino_id IN (SELECT id FROM pessoa WHERE excluida_em IS NULL)",
    )
    .bind(id)
    .execute(&state.pool)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::Conflict(
            "vínculo ausente da lixeira ou uma das pessoas também está na lixeira".to_owned(),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn excluir_definitivamente(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    let resultado =
        sqlx::query("DELETE FROM pessoa_vinculo WHERE id = ? AND excluido_em IS NOT NULL")
            .bind(id)
            .execute(&state.pool)
            .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("vínculo na lixeira"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn obter_grafo(State(state): State<AppState>) -> Result<Json<GrafoResponse>, AppError> {
    let mut tx = state.pool.begin().await?;
    let node_rows = sqlx::query_as::<_, GrafoNodeLinha>(
        "SELECT p.id, p.nome AS label, COALESCE(c.cor_hex, '#86A6A3') AS color, \
                CASE WHEN p.foto_principal IS NOT NULL \
                    THEN '/api/dossie/pessoas/' || p.id || '/foto' \
                    ELSE NULL \
                END AS foto_url, \
                c.nome_categoria AS categoria, p.pessoa_juridica, p.descricao, p.classificacao_risco, p.toxicidade \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         WHERE p.excluida_em IS NULL \
         ORDER BY p.nome COLLATE NOCASE",
    )
    .fetch_all(&mut *tx)
    .await?;
    let contact_rows = sqlx::query_as::<_, GrafoContatoLinha>(
        "SELECT co.pessoa_id, t.nome_tipo AS tipo, co.valor \
         FROM contato co \
         JOIN tipo_meio_contato t ON t.id = co.tipo_contato_id \
         ORDER BY co.pessoa_id, co.id",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut contacts_by_person = HashMap::<i64, Vec<GrafoContato>>::new();
    for contact in contact_rows {
        contacts_by_person
            .entry(contact.pessoa_id)
            .or_default()
            .push(GrafoContato {
                tipo: contact.tipo,
                valor: contact.valor,
            });
    }
    let (hp_configuracao, mut indicadores) =
        super::hp_psicossocial::calcular_snapshot(&mut tx).await?;
    let nodes = node_rows
        .into_iter()
        .map(|node| {
            Ok(GrafoNode {
                id: node.id,
                label: node.label,
                color: node.color,
                foto_url: node.foto_url,
                categoria: node.categoria,
                pessoa_juridica: node.pessoa_juridica,
                descricao: node.descricao,
                contatos: contacts_by_person.remove(&node.id).unwrap_or_default(),
                classificacao_risco: node.classificacao_risco,
                toxicidade: node.toxicidade,
                psicossocial: indicadores
                    .remove(&node.id)
                    .ok_or_else(|| AppError::interno("nó ausente do snapshot de HP"))?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let edges = sqlx::query_as::<_, GrafoEdge>(
        "SELECT id, pessoa_origem_id AS source, pessoa_destino_id AS target, \
                tipo_vinculo AS label, descricao, data_criacao \
         FROM pessoa_vinculo WHERE excluido_em IS NULL AND pessoa_origem_id IN (SELECT id FROM pessoa WHERE excluida_em IS NULL) AND pessoa_destino_id IN (SELECT id FROM pessoa WHERE excluida_em IS NULL) ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(GrafoResponse {
        nodes,
        edges,
        hp_configuracao,
    }))
}

#[derive(sqlx::FromRow)]
struct GrafoNodeLinha {
    id: i64,
    label: String,
    color: String,
    foto_url: Option<String>,
    categoria: Option<String>,
    pessoa_juridica: bool,
    descricao: Option<String>,
    classificacao_risco: String,
    toxicidade: f64,
}

#[derive(sqlx::FromRow)]
struct GrafoContatoLinha {
    pessoa_id: i64,
    tipo: String,
    valor: String,
}

async fn buscar_vinculo(state: &AppState, id: i64) -> Result<PessoaVinculo, AppError> {
    sqlx::query_as::<_, PessoaVinculo>(
        "SELECT id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao \
         FROM pessoa_vinculo WHERE id = ? AND excluido_em IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("vínculo"))
}

fn validar_vinculo(input: &VinculoInput) -> Result<(), AppError> {
    if input.pessoa_origem_id <= 0 || input.pessoa_destino_id <= 0 {
        return Err(AppError::BadRequest(
            "pessoa_origem_id e pessoa_destino_id são obrigatórios".to_owned(),
        ));
    }
    if input.pessoa_origem_id == input.pessoa_destino_id {
        return Err(AppError::BadRequest(
            "uma pessoa não pode ter vínculo consigo mesma".to_owned(),
        ));
    }
    if input.tipo_vinculo.trim().is_empty() {
        return Err(AppError::BadRequest(
            "tipo_vinculo é obrigatório".to_owned(),
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

#[derive(sqlx::FromRow)]
struct AnexoVinculoLinha {
    id: i64,
    vinculo_id: i64,
    nome_arquivo: String,
    mime_type: String,
    tamanho_bytes: i64,
    data_upload: String,
}

impl From<AnexoVinculoLinha> for AnexoVinculoResumo {
    fn from(anexo: AnexoVinculoLinha) -> Self {
        let url_thumbnail = super::miniaturas::mime_suportado(&anexo.mime_type)
            .then(|| format!("/api/vinculos/anexos/{}/thumbnail", anexo.id));
        Self {
            id: anexo.id,
            vinculo_id: anexo.vinculo_id,
            nome_arquivo: anexo.nome_arquivo,
            mime_type: anexo.mime_type,
            tamanho_bytes: anexo.tamanho_bytes,
            data_upload: anexo.data_upload,
            url_stream: format!("/api/vinculos/anexos/{}/stream", anexo.id),
            url_download: format!("/api/vinculos/anexos/{}/download", anexo.id),
            url_thumbnail,
        }
    }
}
