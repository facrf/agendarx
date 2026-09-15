use std::{
    io::{Cursor, Read, Write},
    path::{Path as FsPath, PathBuf},
};

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::Response,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Acquire;
use uuid::Uuid;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/backups", get(listar).post(criar_manual))
        .route("/backups/{id}", axum::routing::delete(excluir))
        .route("/backups/{id}/download", get(download))
        .route("/exportacao-segura", post(exportar_seguro))
        .route("/restaurar", post(restaurar))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BackupInfo {
    pub id: i64,
    pub nome_arquivo: String,
    pub tamanho_bytes: i64,
    pub automatico: bool,
    pub data_criacao: String,
}

#[derive(Deserialize)]
struct ExportInput {
    senha: String,
}

fn exigir_admin(sessao: &SessaoAutenticada) -> Result<(), AppError> {
    if sessao.usuario.perfil == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn diretorio_backup(state: &AppState) -> Result<PathBuf, AppError> {
    let path = state
        .config
        .database_url
        .strip_prefix("sqlite://")
        .and_then(|v| v.split('?').next())
        .ok_or_else(|| AppError::interno("backup exige banco SQLite em arquivo"))?;
    Ok(FsPath::new(path)
        .parent()
        .unwrap_or_else(|| FsPath::new("."))
        .join("backups"))
}

pub async fn criar_snapshot(state: &AppState, automatico: bool) -> Result<BackupInfo, AppError> {
    let dir = diretorio_backup(state)?;
    std::fs::create_dir_all(&dir)?;
    let nome = format!(
        "agendarx-{}-{}.db",
        Utc::now().format("%Y%m%d-%H%M%S"),
        Uuid::new_v4().simple()
    );
    let path = dir.join(&nome);
    let escaped = path.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(&state.pool)
        .await?;
    let tamanho = i64::try_from(std::fs::metadata(&path)?.len())
        .map_err(|_| AppError::interno("backup grande demais"))?;
    let info = sqlx::query_as("INSERT INTO backup_registro (nome_arquivo, tamanho_bytes, automatico) VALUES (?, ?, ?) RETURNING id, nome_arquivo, tamanho_bytes, automatico, data_criacao")
        .bind(&nome).bind(tamanho).bind(automatico).fetch_one(&state.pool).await?;
    limpar_antigos(state, 7).await?;
    Ok(info)
}

async fn limpar_antigos(state: &AppState, manter_automaticos: i64) -> Result<(), AppError> {
    let antigos: Vec<(i64, String)> = sqlx::query_as("SELECT id, nome_arquivo FROM backup_registro WHERE automatico = 1 ORDER BY id DESC LIMIT -1 OFFSET ?")
        .bind(manter_automaticos).fetch_all(&state.pool).await?;
    let dir = diretorio_backup(state)?;
    for (id, nome) in antigos {
        let _ = std::fs::remove_file(dir.join(&nome));
        sqlx::query("DELETE FROM backup_registro WHERE id = ?")
            .bind(id)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

pub fn iniciar_rotina(state: AppState) {
    tokio::spawn(async move {
        loop {
            if let Err(error) = criar_snapshot(&state, true).await {
                tracing::error!(%error, "backup automático falhou");
            }
            tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
        }
    });
}

async fn listar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<Vec<BackupInfo>>, AppError> {
    exigir_admin(&sessao)?;
    Ok(Json(sqlx::query_as("SELECT id, nome_arquivo, tamanho_bytes, automatico, data_criacao FROM backup_registro ORDER BY id DESC LIMIT 50").fetch_all(&state.pool).await?))
}

async fn criar_manual(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<(StatusCode, Json<BackupInfo>), AppError> {
    exigir_admin(&sessao)?;
    Ok((
        StatusCode::CREATED,
        Json(criar_snapshot(&state, false).await?),
    ))
}

async fn download(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    exigir_admin(&sessao)?;
    let info: BackupInfo = sqlx::query_as("SELECT id, nome_arquivo, tamanho_bytes, automatico, data_criacao FROM backup_registro WHERE id = ?").bind(id).fetch_optional(&state.pool).await?.ok_or_else(|| AppError::nao_encontrado("backup"))?;
    responder_arquivo(
        std::fs::read(diretorio_backup(&state)?.join(&info.nome_arquivo))?,
        &info.nome_arquivo,
        "application/vnd.sqlite3",
    )
}

async fn excluir(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    exigir_admin(&sessao)?;
    let nome: String = sqlx::query_scalar("SELECT nome_arquivo FROM backup_registro WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::nao_encontrado("backup"))?;
    let path = diretorio_backup(&state)?.join(nome);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    sqlx::query("DELETE FROM backup_registro WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn exportar_seguro(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<ExportInput>,
) -> Result<Response, AppError> {
    exigir_admin(&sessao)?;
    if input.senha.chars().count() < 10 {
        return Err(AppError::BadRequest(
            "use uma senha de exportação com pelo menos 10 caracteres".into(),
        ));
    }
    let backup = criar_snapshot(&state, false).await?;
    let db = std::fs::read(diretorio_backup(&state)?.join(&backup.nome_arquivo))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .with_aes_encryption(AesMode::Aes256, &input.senha);
    writer
        .start_file("agendarx.db", options)
        .map_err(|e| AppError::interno(e.to_string()))?;
    writer.write_all(&db)?;
    let bytes = writer
        .finish()
        .map_err(|e| AppError::interno(e.to_string()))?
        .into_inner();
    responder_arquivo(
        bytes,
        &format!("agendarx-seguro-{}.zip", Utc::now().format("%Y%m%d-%H%M%S")),
        "application/zip",
    )
}

async fn restaurar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_admin(&sessao)?;
    let mut arquivo = None;
    let mut senha = String::new();
    while let Some(field) = multipart.next_field().await.map_err(AppError::from)? {
        match field.name() {
            Some("arquivo") => {
                arquivo = Some(field.bytes().await.map_err(AppError::from)?.to_vec())
            }
            Some("senha") => senha = field.text().await.map_err(AppError::from)?,
            _ => {}
        }
    }
    let bytes = arquivo.ok_or_else(|| AppError::BadRequest("selecione um backup".into()))?;
    if bytes.len() > state.config.task_storage_quota_bytes as usize {
        return Err(AppError::PayloadTooLarge);
    }
    let db = extrair_banco(
        &bytes,
        &senha,
        state.config.task_storage_quota_bytes as usize,
    )?;
    let seguranca = criar_snapshot(&state, false).await?;
    restaurar_banco(&state, &db).await?;
    Ok(Json(
        serde_json::json!({"mensagem":"backup restaurado", "backup_seguranca":seguranca.nome_arquivo}),
    ))
}

fn extrair_banco(bytes: &[u8], senha: &str, max_bytes: usize) -> Result<Vec<u8>, AppError> {
    if bytes.starts_with(b"SQLite format 3\0") {
        if bytes.len() > max_bytes {
            return Err(AppError::PayloadTooLarge);
        }
        return Ok(bytes.to_vec());
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| {
        AppError::BadRequest("o arquivo não é um backup SQLite ou ZIP válido".into())
    })?;
    let index = (0..zip.len())
        .find(|i| zip.file_names().nth(*i).is_some_and(|n| n.ends_with(".db")))
        .ok_or_else(|| AppError::BadRequest("o ZIP não contém agendarx.db".into()))?;
    let mut file = if senha.is_empty() {
        zip.by_index(index)
    } else {
        zip.by_index_decrypt(index, senha.as_bytes())
    }
    .map_err(|_| AppError::BadRequest("senha incorreta ou ZIP inválido".into()))?;
    if file.size() > max_bytes as u64 {
        return Err(AppError::PayloadTooLarge);
    }
    let mut db = Vec::new();
    file.read_to_end(&mut db)
        .map_err(|_| AppError::BadRequest("não foi possível descriptografar o backup".into()))?;
    if !db.starts_with(b"SQLite format 3\0") {
        return Err(AppError::BadRequest(
            "o conteúdo do backup não é SQLite".into(),
        ));
    }
    Ok(db)
}

async fn restaurar_banco(state: &AppState, bytes: &[u8]) -> Result<(), AppError> {
    let dir = diretorio_backup(state)?;
    let path = dir.join(format!("restauracao-{}.db", Uuid::new_v4().simple()));
    std::fs::write(&path, bytes)?;
    let mut conn = state.pool.acquire().await?;
    sqlx::query("ATTACH DATABASE ? AS restauracao")
        .bind(path.to_string_lossy().as_ref())
        .execute(&mut *conn)
        .await?;
    let required: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM restauracao.sqlite_master WHERE type='table' AND name IN ('pessoa','anexo_dossie','etiqueta')").fetch_one(&mut *conn).await?;
    if required != 3 {
        let _ = sqlx::query("DETACH DATABASE restauracao")
            .execute(&mut *conn)
            .await;
        let _ = std::fs::remove_file(&path);
        return Err(AppError::BadRequest(
            "backup incompatível com esta versão do AgendarX".into(),
        ));
    }
    let mut tx = conn.begin().await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *tx)
        .await?;
    let delete_order = [
        "miniatura_anexo_tarefa_calendario",
        "historico_tarefa_calendario",
        "anexo_tarefa_calendario",
        "tarefa_calendario_pessoa",
        "tarefa_calendario",
        "historico_busca_publica",
        "parametro_busca",
        "miniatura_anexo_vinculo",
        "anexo_vinculo",
        "miniatura_anexo_dossie",
        "anexo_dossie",
        "pessoa_etiqueta",
        "pessoa_favorita",
        "posicao_grafo",
        "pessoa_vinculo",
        "contato",
        "pessoa",
        "etiqueta",
        "categoria_pessoa",
        "tipo_meio_contato",
        "identidade_visual",
    ];
    for table in delete_order {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
    }
    let insert_order = [
        "categoria_pessoa",
        "tipo_meio_contato",
        "pessoa",
        "contato",
        "pessoa_vinculo",
        "anexo_dossie",
        "miniatura_anexo_dossie",
        "anexo_vinculo",
        "miniatura_anexo_vinculo",
        "parametro_busca",
        "historico_busca_publica",
        "tarefa_calendario",
        "tarefa_calendario_pessoa",
        "anexo_tarefa_calendario",
        "miniatura_anexo_tarefa_calendario",
        "historico_tarefa_calendario",
        "etiqueta",
        "pessoa_etiqueta",
        "pessoa_favorita",
        "posicao_grafo",
        "identidade_visual",
    ];
    for table in insert_order {
        sqlx::query(&format!(
            "INSERT INTO main.{table} SELECT * FROM restauracao.{table}"
        ))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    sqlx::query("DETACH DATABASE restauracao")
        .execute(&mut *conn)
        .await?;
    std::fs::remove_file(path)?;
    Ok(())
}

fn responder_arquivo(bytes: Vec<u8>, nome: &str, mime: &str) -> Result<Response, AppError> {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{nome}\""),
        )
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(bytes))
        .map_err(|e| AppError::interno(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{criar_snapshot, diretorio_backup, extrair_banco, restaurar_banco};
    use crate::{AppState, config::Config, db};
    use std::io::{Cursor, Write};
    use zip::{AesMode, ZipWriter, write::SimpleFileOptions};

    #[test]
    fn abre_exportacao_aes_com_senha_correta() {
        let sqlite = b"SQLite format 3\0dados";
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "agendarx.db",
                SimpleFileOptions::default().with_aes_encryption(AesMode::Aes256, "senha-segura"),
            )
            .unwrap();
        writer.write_all(sqlite).unwrap();
        let zip = writer.finish().unwrap().into_inner();
        assert_eq!(extrair_banco(&zip, "senha-segura", 1_000).unwrap(), sqlite);
        assert!(extrair_banco(&zip, "senha-incorreta", 1_000).is_err());
    }

    #[tokio::test]
    async fn snapshot_e_restauracao_recuperam_dados() {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let mut config = Config::from_env().unwrap();
        config.database_url = format!("sqlite://.cache/backup-test-{id}.db?mode=rwc");
        config.admin_login = None;
        config.admin_password = None;
        let pool = db::conectar(&config).await.unwrap();
        let state = AppState {
            pool: pool.clone(),
            config,
        };
        sqlx::query("INSERT INTO pessoa (nome) VALUES ('Antes')")
            .execute(&pool)
            .await
            .unwrap();
        let backup = criar_snapshot(&state, false).await.unwrap();
        let bytes =
            std::fs::read(diretorio_backup(&state).unwrap().join(&backup.nome_arquivo)).unwrap();
        sqlx::query("UPDATE pessoa SET nome='Depois'")
            .execute(&pool)
            .await
            .unwrap();
        restaurar_banco(&state, &bytes).await.unwrap();
        let nome: String = sqlx::query_scalar("SELECT nome FROM pessoa")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(nome, "Antes");
        pool.close().await;
        let _ = std::fs::remove_file(format!(".cache/backup-test-{id}.db"));
        let _ = std::fs::remove_file(diretorio_backup(&state).unwrap().join(backup.nome_arquivo));
    }
}
