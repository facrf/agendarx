use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Multipart, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};

use crate::{
    AppState,
    error::AppError,
    models::{AnexoDossie, AnexoResumo, MensagemResponse},
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route(
            "/pessoas/{pessoa_id}/anexos",
            get(listar_anexos).post(enviar_anexo),
        )
        .route("/anexos/{id}", get(obter_metadados).delete(excluir_anexo))
        .route("/anexos/{id}/stream", get(stream_anexo))
        .route("/anexos/{id}/download", get(download_anexo))
        .route(
            "/pessoas/{pessoa_id}/foto",
            get(obter_foto).put(atualizar_foto).delete(excluir_foto),
        )
}

async fn listar_anexos(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<AnexoResumo>>, AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let linhas = sqlx::query_as::<_, AnexoLinha>(
        "SELECT id, pessoa_id, nome_arquivo, mime_type, tamanho_bytes, data_upload \
         FROM anexo_dossie WHERE pessoa_id = ? ORDER BY data_upload DESC, id DESC",
    )
    .bind(pessoa_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(linhas.into_iter().map(AnexoResumo::from).collect()))
}

async fn obter_metadados(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<AnexoResumo>, AppError> {
    let linha = sqlx::query_as::<_, AnexoLinha>(
        "SELECT id, pessoa_id, nome_arquivo, mime_type, tamanho_bytes, data_upload \
         FROM anexo_dossie WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo"))?;
    Ok(Json(linha.into()))
}

async fn enviar_anexo(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<AnexoResumo>), AppError> {
    garantir_pessoa(&state, pessoa_id).await?;
    let mut arquivo = None;

    while let Some(campo) = multipart
        .next_field()
        .await
        .map_err(|erro| AppError::BadRequest(format!("multipart inválido: {erro}")))?
    {
        if campo.file_name().is_none() && campo.name() != Some("arquivo") {
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
    if conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("o arquivo está vazio".to_owned()));
    }
    if nome_arquivo.len() > 255 {
        return Err(AppError::BadRequest(
            "nome do arquivo excede 255 caracteres".to_owned(),
        ));
    }

    let mime_type = mime_informado
        .filter(|mime| mime.parse::<mime::Mime>().is_ok())
        .or_else(|| infer::get(&conteudo).map(|tipo| tipo.mime_type().to_owned()))
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let tamanho = conteudo.len() as i64;
    let linha = sqlx::query_as::<_, AnexoLinha>(
        "INSERT INTO anexo_dossie \
            (pessoa_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes) \
         VALUES (?, ?, ?, ?, ?) \
         RETURNING id, pessoa_id, nome_arquivo, mime_type, tamanho_bytes, data_upload",
    )
    .bind(pessoa_id)
    .bind(nome_arquivo)
    .bind(mime_type)
    .bind(conteudo.as_ref())
    .bind(tamanho)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(linha.into())))
}

async fn stream_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let anexo = buscar_anexo(&state, id).await?;
    Ok(servir_blob(
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
    Ok(servir_blob(
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
    let resultado = sqlx::query("DELETE FROM anexo_dossie WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("anexo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn atualizar_foto(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    headers: HeaderMap,
    conteudo: Bytes,
) -> Result<(StatusCode, Json<MensagemResponse>), AppError> {
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("a imagem está vazia".to_owned()));
    }
    if conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }
    let mime_cabecalho = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let mime_detectado = infer::get(&conteudo).map(|tipo| tipo.mime_type());
    if !mime_cabecalho.starts_with("image/")
        || !mime_detectado.is_some_and(|mime| mime.starts_with("image/"))
    {
        return Err(AppError::BadRequest(
            "a foto deve ser uma imagem reconhecida e usar Content-Type image/*".to_owned(),
        ));
    }

    let resultado = sqlx::query("UPDATE pessoa SET foto_principal = ? WHERE id = ?")
        .bind(conteudo.as_ref())
        .bind(pessoa_id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok((
        StatusCode::OK,
        Json(MensagemResponse {
            mensagem: "foto atualizada".to_owned(),
        }),
    ))
}

async fn obter_foto(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let foto: Option<Vec<u8>> =
        sqlx::query_scalar::<_, Option<Vec<u8>>>("SELECT foto_principal FROM pessoa WHERE id = ?")
            .bind(pessoa_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::nao_encontrado("pessoa"))?;
    let foto = foto.ok_or_else(|| AppError::nao_encontrado("foto"))?;
    let mime = infer::get(&foto)
        .map(|tipo| tipo.mime_type())
        .unwrap_or("application/octet-stream");
    Ok(servir_blob(
        foto,
        mime,
        "foto",
        false,
        headers.get(header::RANGE).and_then(|v| v.to_str().ok()),
    ))
}

async fn excluir_foto(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let resultado = sqlx::query(
        "UPDATE pessoa SET foto_principal = NULL WHERE id = ? AND foto_principal IS NOT NULL",
    )
    .bind(pessoa_id)
    .execute(&state.pool)
    .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa ou foto"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn buscar_anexo(state: &AppState, id: i64) -> Result<AnexoDossie, AppError> {
    sqlx::query_as::<_, AnexoDossie>(
        "SELECT id, pessoa_id, nome_arquivo, mime_type, conteudo_blob, tamanho_bytes, data_upload \
         FROM anexo_dossie WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo"))
}

async fn garantir_pessoa(state: &AppState, id: i64) -> Result<(), AppError> {
    let existe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pessoa WHERE id = ?)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if !existe {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    Ok(())
}

fn servir_blob(
    dados: Vec<u8>,
    mime_type: &str,
    nome_arquivo: &str,
    download: bool,
    range: Option<&str>,
) -> Response {
    let tamanho_total = dados.len();
    let intervalo = range.and_then(|valor| parse_range(valor, tamanho_total));

    if range.is_some() && intervalo.is_none() {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
        if let Ok(valor) = HeaderValue::from_str(&format!("bytes */{tamanho_total}")) {
            response.headers_mut().insert(header::CONTENT_RANGE, valor);
        }
        return response;
    }

    let (inicio, fim, status) = intervalo
        .map(|(inicio, fim)| (inicio, fim, StatusCode::PARTIAL_CONTENT))
        .unwrap_or_else(|| (0, tamanho_total.saturating_sub(1), StatusCode::OK));
    let corpo = if tamanho_total == 0 {
        Vec::new()
    } else {
        dados[inicio..=fim].to_vec()
    };

    let mut response = Response::new(Body::from(corpo));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(
            &(fim.saturating_sub(inicio) + usize::from(tamanho_total > 0)).to_string(),
        )
        .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    let disposicao = if download { "attachment" } else { "inline" };
    let nome_seguro: String = nome_arquivo
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || ".-_ ".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if let Ok(valor) = HeaderValue::from_str(&format!("{disposicao}; filename=\"{nome_seguro}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, valor);
    }
    if status == StatusCode::PARTIAL_CONTENT
        && let Ok(valor) = HeaderValue::from_str(&format!("bytes {inicio}-{fim}/{tamanho_total}"))
    {
        headers.insert(header::CONTENT_RANGE, valor);
    }
    response
}

fn parse_range(valor: &str, tamanho: usize) -> Option<(usize, usize)> {
    if tamanho == 0 {
        return None;
    }
    let valor = valor.strip_prefix("bytes=")?;
    if valor.contains(',') {
        return None;
    }
    let (inicio, fim) = valor.split_once('-')?;
    if inicio.is_empty() {
        let sufixo = fim.parse::<usize>().ok()?.min(tamanho);
        if sufixo == 0 {
            return None;
        }
        return Some((tamanho - sufixo, tamanho - 1));
    }
    let inicio = inicio.parse::<usize>().ok()?;
    if inicio >= tamanho {
        return None;
    }
    let fim = if fim.is_empty() {
        tamanho - 1
    } else {
        fim.parse::<usize>().ok()?.min(tamanho - 1)
    };
    (inicio <= fim).then_some((inicio, fim))
}

#[derive(sqlx::FromRow)]
struct AnexoLinha {
    id: i64,
    pessoa_id: i64,
    nome_arquivo: String,
    mime_type: String,
    tamanho_bytes: i64,
    data_upload: String,
}

impl From<AnexoLinha> for AnexoResumo {
    fn from(anexo: AnexoLinha) -> Self {
        Self {
            id: anexo.id,
            pessoa_id: anexo.pessoa_id,
            nome_arquivo: anexo.nome_arquivo,
            mime_type: anexo.mime_type,
            tamanho_bytes: anexo.tamanho_bytes,
            data_upload: anexo.data_upload,
            url_stream: format!("/api/dossie/anexos/{}/stream", anexo.id),
            url_download: format!("/api/dossie/anexos/{}/download", anexo.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_range;

    #[test]
    fn interpreta_ranges_http() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 1000), None);
    }
}
