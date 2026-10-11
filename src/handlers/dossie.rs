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
    models::{AnexoDossie, AnexoNomeInput, AnexoResumo, MensagemResponse},
};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route(
            "/pessoas/{pessoa_id}/anexos",
            get(listar_anexos).post(enviar_anexo),
        )
        .route(
            "/anexos/{id}",
            get(obter_metadados)
                .put(atualizar_nome_anexo)
                .delete(excluir_anexo),
        )
        .route("/anexos/{id}/stream", get(stream_anexo))
        .route("/anexos/{id}/notas", get(obter_notas).put(salvar_notas))
        .route("/anexos/{id}/download", get(download_anexo))
        .route("/anexos/{id}/thumbnail", get(obter_miniatura))
        .route(
            "/pessoas/{pessoa_id}/foto",
            get(obter_foto)
                .put(atualizar_foto)
                .post(enviar_foto)
                .delete(excluir_foto),
        )
}

async fn obter_notas(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<super::notas::NotasResponse>, AppError> {
    super::notas::obter(&state, super::notas::AnexoTipo::Dossie, id).await
}

async fn salvar_notas(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<super::notas::NotasInput>,
) -> Result<Json<super::notas::NotasInput>, AppError> {
    super::notas::salvar(&state, super::notas::AnexoTipo::Dossie, id, input).await
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

    while let Some(campo) = multipart.next_field().await.map_err(AppError::from)? {
        if campo.name() != Some("arquivo") {
            continue;
        }
        let nome_arquivo = campo.file_name().unwrap_or("arquivo.bin").to_owned();
        let mime_informado = campo.content_type().map(str::to_owned);
        let conteudo = super::upload::ler_campo(campo, state.config.max_upload_bytes).await?;
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
        tracing::warn!(pessoa_id, "upload de foto vazio rejeitado");
        return Err(AppError::BadRequest("o arquivo está vazio".to_owned()));
    }
    let nome_arquivo = normalizar_nome_arquivo(&nome_arquivo)?;

    let mime_type = infer::get(&conteudo)
        .map(|tipo| tipo.mime_type().to_owned())
        .or_else(|| mime_informado.filter(|mime| mime.parse::<mime::Mime>().is_ok()))
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let miniatura = if super::miniaturas::mime_suportado(&mime_type) {
        super::miniaturas::gerar(conteudo.clone()).await?
    } else {
        None
    };
    let tamanho = conteudo.len() as i64;
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let ativa: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pessoa WHERE id=? AND excluida_em IS NULL)",
    )
    .bind(pessoa_id)
    .fetch_one(&mut *tx)
    .await?;
    if !ativa {
        return Err(AppError::nao_encontrado("pessoa ativa"));
    }
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
    .fetch_one(&mut *tx)
    .await?;
    if let Some(miniatura) = miniatura {
        sqlx::query(
            "INSERT INTO miniatura_anexo_dossie \
                (anexo_id, conteudo_webp, largura, altura, tamanho_bytes) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(linha.id)
        .bind(&miniatura.conteudo)
        .bind(i64::from(miniatura.largura))
        .bind(i64::from(miniatura.altura))
        .bind(miniatura.conteudo.len() as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(linha.into())))
}

async fn atualizar_nome_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<AnexoNomeInput>,
) -> Result<Json<AnexoResumo>, AppError> {
    let nome_arquivo = normalizar_nome_arquivo(&input.nome_arquivo)?;
    let linha = sqlx::query_as::<_, AnexoLinha>(
        "UPDATE anexo_dossie SET nome_arquivo = ? WHERE id = ? \
         RETURNING id, pessoa_id, nome_arquivo, mime_type, tamanho_bytes, data_upload",
    )
    .bind(nome_arquivo)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("anexo"))?;
    Ok(Json(linha.into()))
}

async fn stream_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    servir_anexo_banco(&state.pool, OrigemAnexo::Dossie, id, &headers, false).await
}

async fn download_anexo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    servir_anexo_banco(&state.pool, OrigemAnexo::Dossie, id, &headers, true).await
}

async fn obter_miniatura(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    if let Some(conteudo) = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT conteudo_webp FROM miniatura_anexo_dossie WHERE anexo_id = ?",
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
        "INSERT INTO miniatura_anexo_dossie \
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
    let resultado = sqlx::query("DELETE FROM anexo_dossie WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("anexo"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn enviar_foto(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<MensagemResponse>), AppError> {
    while let Some(campo) = multipart.next_field().await.map_err(AppError::from)? {
        if campo.name() == Some("arquivo") {
            let conteudo = super::upload::ler_campo(campo, state.config.max_upload_bytes).await?;
            return atualizar_foto(State(state), Path(pessoa_id), conteudo).await;
        }
    }
    Err(AppError::BadRequest(
        "envie a foto no campo multipart 'arquivo'".to_owned(),
    ))
}

async fn atualizar_foto(
    State(state): State<AppState>,
    Path(pessoa_id): Path<i64>,
    conteudo: Bytes,
) -> Result<(StatusCode, Json<MensagemResponse>), AppError> {
    if conteudo.is_empty() {
        return Err(AppError::BadRequest("a imagem está vazia".to_owned()));
    }
    if conteudo.len() > state.config.max_upload_bytes {
        tracing::warn!(
            pessoa_id,
            tamanho_bytes = conteudo.len(),
            "upload de foto excedeu o limite"
        );
        return Err(AppError::PayloadTooLarge);
    }
    let mime_detectado = infer::get(&conteudo).map(|tipo| tipo.mime_type());
    if !mime_detectado.is_some_and(|mime| mime.starts_with("image/")) {
        tracing::warn!(pessoa_id, ?mime_detectado, "formato de foto rejeitado");
        return Err(AppError::BadRequest(
            "a foto deve ser uma imagem reconhecida".to_owned(),
        ));
    }

    let resultado =
        sqlx::query("UPDATE pessoa SET foto_principal = ? WHERE id = ? AND excluida_em IS NULL")
            .bind(conteudo.as_ref())
            .bind(pessoa_id)
            .execute(&state.pool)
            .await?;
    if resultado.rows_affected() == 0 {
        return Err(AppError::nao_encontrado("pessoa"));
    }
    tracing::info!(
        pessoa_id,
        tamanho_bytes = conteudo.len(),
        ?mime_detectado,
        "foto principal atualizada"
    );
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
    let foto: Option<Vec<u8>> = sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT foto_principal FROM pessoa WHERE id = ? AND excluida_em IS NULL",
    )
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

pub(super) fn normalizar_nome_arquivo(nome: &str) -> Result<String, AppError> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err(AppError::BadRequest(
            "nome_arquivo é obrigatório".to_owned(),
        ));
    }
    if nome.chars().count() > 255 {
        return Err(AppError::BadRequest(
            "nome do arquivo excede 255 caracteres".to_owned(),
        ));
    }
    if nome.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "nome do arquivo contém caracteres inválidos".to_owned(),
        ));
    }
    Ok(nome.to_owned())
}

pub(super) fn servir_blob(
    dados: Vec<u8>,
    mime_type: &str,
    nome_arquivo: &str,
    download: bool,
    range: Option<&str>,
) -> Response {
    let tamanho_total = dados.len();
    let intervalo = range.and_then(|valor| parse_range(valor, tamanho_total));
    if range.is_some() && intervalo.is_none() {
        return range_nao_satisfatorio(tamanho_total);
    }
    let dados = Bytes::from(dados);
    let corpo = intervalo.map_or_else(|| dados.clone(), |(inicio, fim)| dados.slice(inicio..=fim));
    responder_blob(
        corpo,
        tamanho_total,
        intervalo,
        mime_type,
        nome_arquivo,
        download,
    )
}

#[derive(Clone, Copy)]
pub(super) enum OrigemAnexo {
    Dossie,
    Vinculo,
    Tarefa { usuario_id: i64 },
}

impl OrigemAnexo {
    fn fonte(self) -> &'static str {
        match self {
            Self::Dossie => "anexo_dossie a WHERE a.id = ?",
            Self::Vinculo => {
                "anexo_vinculo a JOIN pessoa_vinculo v ON v.id = a.vinculo_id WHERE a.id = ? AND v.excluido_em IS NULL"
            }
            Self::Tarefa { .. } => {
                "anexo_tarefa_calendario a JOIN tarefa_calendario t ON t.id = a.tarefa_id WHERE a.id = ? AND t.usuario_id = ?"
            }
        }
    }
}

pub(super) async fn servir_anexo_banco(
    pool: &sqlx::SqlitePool,
    origem: OrigemAnexo,
    id: i64,
    headers: &HeaderMap,
    download: bool,
) -> Result<Response, AppError> {
    // Keep metadata and bytes in the same SQLite snapshot, including ownership.
    let mut tx = pool.begin().await?;
    let sql = format!(
        "SELECT a.mime_type, a.nome_arquivo, length(a.conteudo_blob) FROM {}",
        origem.fonte()
    );
    let mut consulta = sqlx::query_as::<_, (String, String, i64)>(&sql).bind(id);
    if let OrigemAnexo::Tarefa { usuario_id } = origem {
        consulta = consulta.bind(usuario_id);
    }
    let (mime, nome, tamanho) = consulta
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::nao_encontrado("anexo"))?;
    let tamanho =
        usize::try_from(tamanho).map_err(|_| AppError::interno("tamanho de anexo inválido"))?;
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let intervalo = range.and_then(|v| parse_range(v, tamanho));
    if range.is_some() && intervalo.is_none() {
        tx.commit().await?;
        return Ok(range_nao_satisfatorio(tamanho));
    }
    let sql = format!(
        "SELECT {} FROM {}",
        if intervalo.is_some() {
            "substr(a.conteudo_blob, ?, ?)"
        } else {
            "a.conteudo_blob"
        },
        origem.fonte()
    );
    let mut consulta = sqlx::query_scalar::<_, Vec<u8>>(&sql);
    if let Some((inicio, fim)) = intervalo {
        // SQLite BLOB substr uses a one-based byte offset.
        consulta = consulta
            .bind((inicio + 1) as i64)
            .bind((fim - inicio + 1) as i64);
    }
    consulta = consulta.bind(id);
    if let OrigemAnexo::Tarefa { usuario_id } = origem {
        consulta = consulta.bind(usuario_id);
    }
    let dados = consulta.fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(responder_blob(
        Bytes::from(dados),
        tamanho,
        intervalo,
        &mime,
        &nome,
        download,
    ))
}

fn range_nao_satisfatorio(tamanho_total: usize) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    if let Ok(valor) = HeaderValue::from_str(&format!("bytes */{tamanho_total}")) {
        response.headers_mut().insert(header::CONTENT_RANGE, valor);
    }
    response
}

fn responder_blob(
    corpo: Bytes,
    tamanho_total: usize,
    intervalo: Option<(usize, usize)>,
    mime_type: &str,
    nome_arquivo: &str,
    download: bool,
) -> Response {
    let tamanho_corpo = corpo.len();
    let mut response = Response::new(Body::from(corpo));
    *response.status_mut() = if intervalo.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("sandbox; default-src 'none'"),
    );
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&tamanho_corpo.to_string()).expect("numeric content-length"),
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
    if let Some((inicio, fim)) = intervalo
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
        let url_thumbnail = super::miniaturas::mime_suportado(&anexo.mime_type)
            .then(|| format!("/api/dossie/anexos/{}/thumbnail", anexo.id));
        Self {
            id: anexo.id,
            pessoa_id: anexo.pessoa_id,
            nome_arquivo: anexo.nome_arquivo,
            mime_type: anexo.mime_type,
            tamanho_bytes: anexo.tamanho_bytes,
            data_upload: anexo.data_upload,
            url_stream: format!("/api/dossie/anexos/{}/stream", anexo.id),
            url_download: format!("/api/dossie/anexos/{}/download", anexo.id),
            url_thumbnail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalizar_nome_arquivo, parse_range};

    #[tokio::test]
    async fn leitura_parcial_preserva_bytes_cabecalhos_e_propriedade() {
        use super::{OrigemAnexo, servir_anexo_banco};
        use axum::{
            body::to_bytes,
            http::{HeaderMap, HeaderValue, StatusCode, header},
        };
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql("CREATE TABLE anexo_dossie(id INTEGER PRIMARY KEY, mime_type TEXT, nome_arquivo TEXT, conteudo_blob BLOB); CREATE TABLE pessoa_vinculo(id INTEGER PRIMARY KEY, excluido_em TEXT); CREATE TABLE anexo_vinculo(id INTEGER PRIMARY KEY, vinculo_id INTEGER, mime_type TEXT, nome_arquivo TEXT, conteudo_blob BLOB); CREATE TABLE tarefa_calendario(id INTEGER PRIMARY KEY, usuario_id INTEGER); CREATE TABLE anexo_tarefa_calendario(id INTEGER PRIMARY KEY, tarefa_id INTEGER, mime_type TEXT, nome_arquivo TEXT, conteudo_blob BLOB);").execute(&pool).await.unwrap();
        let dados: Vec<u8> = (0..1024 * 1024).map(|v| (v % 251) as u8).collect();
        for tabela in ["anexo_dossie", "anexo_vinculo", "anexo_tarefa_calendario"] {
            let sql = format!(
                "INSERT INTO {tabela}(id,mime_type,nome_arquivo,conteudo_blob) VALUES(1,'video/mp4','video.mp4',?)"
            );
            sqlx::query(&sql).bind(&dados).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql("INSERT INTO pessoa_vinculo VALUES(1,NULL); UPDATE anexo_vinculo SET vinculo_id=1; INSERT INTO tarefa_calendario VALUES(1,7); UPDATE anexo_tarefa_calendario SET tarefa_id=1;").execute(&pool).await.unwrap();
        for origem in [
            OrigemAnexo::Dossie,
            OrigemAnexo::Vinculo,
            OrigemAnexo::Tarefa { usuario_id: 7 },
        ] {
            for (range, inicio, fim) in [
                ("bytes=500000-500099", 500000, 500099),
                ("bytes=-10", dados.len() - 10, dados.len() - 1),
                ("bytes=1048570-", 1048570, dados.len() - 1),
            ] {
                let mut headers = HeaderMap::new();
                headers.insert(header::RANGE, HeaderValue::from_static(range));
                let resposta = servir_anexo_banco(&pool, origem, 1, &headers, false)
                    .await
                    .unwrap();
                assert_eq!(resposta.status(), StatusCode::PARTIAL_CONTENT);
                assert_eq!(
                    resposta.headers()[header::CONTENT_RANGE],
                    format!("bytes {inicio}-{fim}/{}", dados.len())
                );
                assert_eq!(
                    resposta.headers()[header::CONTENT_LENGTH],
                    (fim - inicio + 1).to_string()
                );
                assert_eq!(
                    resposta.headers()[header::CACHE_CONTROL],
                    "private, no-store"
                );
                assert_eq!(
                    resposta.headers()[header::X_CONTENT_TYPE_OPTIONS],
                    "nosniff"
                );
                assert_eq!(
                    to_bytes(resposta.into_body(), 100).await.unwrap().as_ref(),
                    &dados[inicio..=fim]
                );
            }
            let resposta = servir_anexo_banco(&pool, origem, 1, &HeaderMap::new(), true)
                .await
                .unwrap();
            assert_eq!(resposta.status(), StatusCode::OK);
            assert_eq!(
                resposta.headers()[header::CONTENT_DISPOSITION],
                "attachment; filename=\"video.mp4\""
            );
            assert_eq!(
                to_bytes(resposta.into_body(), dados.len())
                    .await
                    .unwrap()
                    .as_ref(),
                dados
            );
            let mut headers = HeaderMap::new();
            headers.insert(header::RANGE, HeaderValue::from_static("bytes=1048576-"));
            let resposta = servir_anexo_banco(&pool, origem, 1, &headers, false)
                .await
                .unwrap();
            assert_eq!(resposta.status(), StatusCode::RANGE_NOT_SATISFIABLE);
            assert_eq!(resposta.headers()[header::CONTENT_RANGE], "bytes */1048576");
        }
        assert!(matches!(
            servir_anexo_banco(
                &pool,
                OrigemAnexo::Tarefa { usuario_id: 8 },
                1,
                &HeaderMap::new(),
                false
            )
            .await,
            Err(crate::error::AppError::NotFound(_))
        ));
        sqlx::query("UPDATE pessoa_vinculo SET excluido_em=CURRENT_TIMESTAMP")
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            servir_anexo_banco(&pool, OrigemAnexo::Vinculo, 1, &HeaderMap::new(), false).await,
            Err(crate::error::AppError::NotFound(_))
        ));
        assert!(matches!(
            servir_anexo_banco(&pool, OrigemAnexo::Dossie, 99, &HeaderMap::new(), false).await,
            Err(crate::error::AppError::NotFound(_))
        ));
    }

    #[test]
    fn interpreta_ranges_http() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 1000), None);
    }

    #[test]
    fn valida_nome_de_anexo() {
        assert_eq!(normalizar_nome_arquivo(" foto.png ").unwrap(), "foto.png");
        assert!(normalizar_nome_arquivo("   ").is_err());
        assert!(normalizar_nome_arquivo("nome\nquebrado.jpg").is_err());
    }
}
