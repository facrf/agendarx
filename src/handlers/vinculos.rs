use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};

use crate::{
    AppState,
    error::AppError,
    models::{
        AnexoVinculo, AnexoVinculoResumo, GrafoEdge, GrafoNode, GrafoResponse, PessoaVinculo,
        VinculoInput,
    },
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/", get(listar_vinculos).post(criar_vinculo))
        .route("/grafo", get(obter_grafo))
        .route(
            "/{vinculo_id}/anexos",
            get(listar_anexos).post(enviar_anexo),
        )
        .route(
            "/anexos/{id}",
            get(obter_metadados_anexo).delete(excluir_anexo),
        )
        .route("/anexos/{id}/stream", get(stream_anexo))
        .route("/anexos/{id}/download", get(download_anexo))
        .route(
            "/{id}",
            get(obter_vinculo)
                .put(atualizar_vinculo)
                .delete(excluir_vinculo),
        )
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

    while let Some(campo) = multipart
        .next_field()
        .await
        .map_err(|erro| AppError::BadRequest(format!("multipart inválido: {erro}")))?
    {
        if campo.name() != Some("arquivo") {
            continue;
        }
        let nome_arquivo = campo.file_name().unwrap_or("arquivo.bin").to_owned();
        let mime_informado = campo.content_type().map(str::to_owned);
        let conteudo = campo
            .bytes()
            .await
            .map_err(|erro| AppError::BadRequest(format!("falha no upload: {erro}")))?;
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
    if nome_arquivo.chars().count() > 255 {
        return Err(AppError::BadRequest(
            "nome do arquivo excede 255 caracteres".to_owned(),
        ));
    }

    let mime_type = infer::get(&conteudo)
        .map(|tipo| tipo.mime_type().to_owned())
        .or_else(|| mime_informado.filter(|mime| mime.parse::<mime::Mime>().is_ok()))
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let tamanho_bytes = conteudo.len() as i64;
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
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(anexo.into())))
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

async fn excluir_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
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
    sqlx::query_as::<_, AnexoVinculo>(
        "SELECT id, vinculo_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes, data_upload \
         FROM anexo_vinculo WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo do vínculo"))
}

async fn listar_vinculos(
    State(state): State<AppState>,
) -> Result<Json<Vec<PessoaVinculo>>, AppError> {
    let vinculos = sqlx::query_as::<_, PessoaVinculo>(
        "SELECT id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao \
         FROM pessoa_vinculo ORDER BY data_criacao DESC, id DESC",
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
         WHERE id = ? \
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
    let resultado = sqlx::query("DELETE FROM pessoa_vinculo WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("vínculo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn obter_grafo(State(state): State<AppState>) -> Result<Json<GrafoResponse>, AppError> {
    let nodes = sqlx::query_as::<_, GrafoNode>(
        "SELECT p.id, p.nome AS label, COALESCE(c.cor_hex, '#86A6A3') AS color, \
                CASE WHEN p.foto_principal IS NOT NULL \
                    THEN '/api/dossie/pessoas/' || p.id || '/foto' \
                    ELSE NULL \
                END AS foto_url, \
                c.nome_categoria AS categoria \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         ORDER BY p.nome COLLATE NOCASE",
    )
    .fetch_all(&state.pool)
    .await?;
    let edges = sqlx::query_as::<_, GrafoEdge>(
        "SELECT id, pessoa_origem_id AS source, pessoa_destino_id AS target, \
                tipo_vinculo AS label, descricao \
         FROM pessoa_vinculo ORDER BY id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(GrafoResponse { nodes, edges }))
}

async fn buscar_vinculo(state: &AppState, id: i64) -> Result<PessoaVinculo, AppError> {
    sqlx::query_as::<_, PessoaVinculo>(
        "SELECT id, pessoa_origem_id, pessoa_destino_id, tipo_vinculo, descricao, data_criacao \
         FROM pessoa_vinculo WHERE id = ?",
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
        Self {
            id: anexo.id,
            vinculo_id: anexo.vinculo_id,
            nome_arquivo: anexo.nome_arquivo,
            mime_type: anexo.mime_type,
            tamanho_bytes: anexo.tamanho_bytes,
            data_upload: anexo.data_upload,
            url_stream: format!("/api/vinculos/anexos/{}/stream", anexo.id),
            url_download: format!("/api/vinculos/anexos/{}/download", anexo.id),
        }
    }
}
