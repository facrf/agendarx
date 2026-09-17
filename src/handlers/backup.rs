use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::{Path as FsPath, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration as StdDuration,
};

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
    routing::{get, post},
};
use chrono::{DateTime, Datelike, Local, NaiveDateTime, NaiveTime, TimeZone, Utc};
use rusqlite::{Connection, DatabaseName, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::{
    io::{AsyncRead, AsyncWriteExt, ReadBuf},
    sync::{Mutex, RwLock},
};
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};

const SCHEMA_ATUAL: i64 = 17;
const FORMATO_BACKUP: u32 = 1;
const EXPIRACAO_RESTORE_MINUTOS: i64 = 30;

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/backups", get(listar).post(criar_manual))
        .route(
            "/backups/configuracao",
            get(obter_configuracao).put(atualizar_configuracao),
        )
        .route("/backups/{id}", axum::routing::delete(excluir))
        .route("/backups/{id}/download", get(download))
        .route("/exportacao-segura", post(exportar_seguro))
        .route("/exportacoes/{token}/download", get(baixar_exportacao))
        .route(
            "/restaurar",
            post(preparar_restauracao).layer(DefaultBodyLimit::disable()),
        )
        .route(
            "/restauracoes/{token}/confirmar",
            post(confirmar_restauracao),
        )
        .route(
            "/restauracoes/{token}",
            axum::routing::delete(cancelar_restauracao),
        )
}

#[derive(Clone, Default)]
pub struct BackupRuntime {
    operacao: Arc<Mutex<()>>,
    manutencao: Arc<RwLock<()>>,
    pendentes: Arc<Mutex<HashMap<String, RestauracaoPendente>>>,
    exportacoes: Arc<Mutex<HashMap<String, ExportacaoPendente>>>,
}

#[derive(Clone)]
struct RestauracaoPendente {
    caminho: PathBuf,
    usuario_id: i64,
    expira_em: DateTime<Utc>,
    previa: RestauracaoPrevia,
}

#[derive(Clone)]
struct ExportacaoPendente {
    caminho: PathBuf,
    nome: String,
    usuario_id: i64,
    expira_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BackupInfo {
    pub id: i64,
    pub nome_arquivo: String,
    pub tamanho_bytes: i64,
    pub automatico: bool,
    pub data_criacao: String,
    pub tipo: String,
    pub sha256: String,
    pub integridade_ok: bool,
    pub versao_app: Option<String>,
    pub schema_versao: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackupConfiguracao {
    pub ativo: bool,
    pub horario: String,
    pub manter_diarios: i64,
    pub manter_semanais: i64,
    pub manter_mensais: i64,
    #[serde(default)]
    pub ultima_execucao_em: Option<String>,
    #[serde(default)]
    pub ultima_tentativa_em: Option<String>,
    #[serde(default)]
    pub ultimo_erro: Option<String>,
}

#[derive(Deserialize)]
struct ExportInput {
    senha: String,
}

#[derive(Serialize)]
struct ExportacaoPronta {
    token: String,
    nome_arquivo: String,
    tamanho_bytes: u64,
    expira_em: String,
}

#[derive(Deserialize)]
struct ConfirmarInput {
    confirmacao: String,
}

#[derive(Debug, Clone, Serialize)]
struct RestauracaoPrevia {
    token: String,
    expira_em: String,
    criado_em: Option<String>,
    versao_app: Option<String>,
    schema_versao: i64,
    schema_atual: i64,
    tamanho_bytes: u64,
    sha256: String,
    pessoas: i64,
    usuarios: i64,
    anexos: i64,
    criptografado: bool,
    avisos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifesto {
    formato: u32,
    aplicacao: String,
    versao: String,
    criado_em: String,
    tamanho_banco: u64,
    sha256: String,
    schema_versao: i64,
    pessoas: i64,
    usuarios: i64,
    anexos: i64,
}

#[derive(Debug, Clone)]
struct ValidacaoBanco {
    schema_versao: i64,
    pessoas: i64,
    usuarios: i64,
    anexos: i64,
}

pub async fn aguardar_manutencao(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, AppError> {
    let caminho = request.uri().path();
    if caminho.starts_with("/api/configuracoes/restauracoes/") && caminho.ends_with("/confirmar") {
        return Ok(next.run(request).await);
    }
    let _leitura = state.backup_runtime.manutencao.read().await;
    Ok(next.run(request).await)
}

fn exigir_admin(sessao: &SessaoAutenticada) -> Result<(), AppError> {
    if sessao.usuario.perfil == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn caminho_banco(state: &AppState) -> Result<PathBuf, AppError> {
    state
        .config
        .database_url
        .strip_prefix("sqlite://")
        .and_then(|valor| valor.split('?').next())
        .filter(|valor| !valor.is_empty() && *valor != ":memory:")
        .map(PathBuf::from)
        .ok_or_else(|| AppError::interno("backup exige banco SQLite em arquivo"))
}

fn diretorio_backup(state: &AppState) -> Result<PathBuf, AppError> {
    Ok(caminho_banco(state)?
        .parent()
        .unwrap_or_else(|| FsPath::new("."))
        .join("backups"))
}

fn conferir_espaco(diretorio: &FsPath, necessario: u64) -> Result<(), AppError> {
    let disponivel = fs2::available_space(diretorio)?;
    let margem = 32 * 1024 * 1024;
    if disponivel < necessario.saturating_add(margem) {
        return Err(AppError::ServiceUnavailable(format!(
            "espaço insuficiente para o backup: são necessários ao menos {} bytes livres",
            necessario.saturating_add(margem)
        )));
    }
    Ok(())
}

pub async fn criar_snapshot(state: &AppState, automatico: bool) -> Result<BackupInfo, AppError> {
    let _operacao =
        state.backup_runtime.operacao.try_lock().map_err(|_| {
            AppError::Conflict("já existe uma operação de backup em andamento".into())
        })?;
    criar_snapshot_interno(state, if automatico { "automatico" } else { "manual" }).await
}

async fn criar_snapshot_interno(state: &AppState, tipo: &str) -> Result<BackupInfo, AppError> {
    let dir = diretorio_backup(state)?;
    std::fs::create_dir_all(&dir)?;
    let banco = caminho_banco(state)?;
    let tamanho_estimado = std::fs::metadata(&banco)?.len();
    conferir_espaco(&dir, tamanho_estimado)?;

    let nome = format!(
        "agendarx-{}-{}-{}.db",
        tipo,
        Utc::now().format("%Y%m%d-%H%M%S"),
        Uuid::new_v4().simple()
    );
    let path = dir.join(&nome);
    let mut snapshot_guard = ArquivoTemporario::new(path.clone());
    let escaped = path.to_string_lossy().replace('\'', "''");
    if let Err(error) = sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(&state.pool)
        .await
    {
        let _ = std::fs::remove_file(&path);
        return Err(error.into());
    }
    restringir_permissoes(&path)?;

    let path_validacao = path.clone();
    let validacao = tokio::task::spawn_blocking(move || validar_banco(&path_validacao))
        .await
        .map_err(|_| AppError::interno("falha ao validar o snapshot"))??;
    let path_hash = path.clone();
    let sha256 = tokio::task::spawn_blocking(move || hash_arquivo(&path_hash))
        .await
        .map_err(|_| AppError::interno("falha ao calcular o hash do snapshot"))??;
    let tamanho = i64::try_from(std::fs::metadata(&path)?.len())
        .map_err(|_| AppError::interno("backup grande demais"))?;
    let automatico = tipo == "automatico";
    let registro = sqlx::query_as::<_, BackupInfo>(
        "INSERT INTO backup_registro (nome_arquivo, tamanho_bytes, automatico, tipo, sha256, integridade_ok, versao_app, schema_versao) \
         VALUES (?, ?, ?, ?, ?, 1, ?, ?) \
         RETURNING id, nome_arquivo, tamanho_bytes, automatico, data_criacao, tipo, sha256, integridade_ok, versao_app, schema_versao",
    )
    .bind(&nome)
    .bind(tamanho)
    .bind(automatico)
    .bind(tipo)
    .bind(sha256)
    .bind(env!("CARGO_PKG_VERSION"))
    .bind(validacao.schema_versao)
    .fetch_one(&state.pool)
    .await;
    let registro = match registro {
        Ok(registro) => registro,
        Err(error) => {
            let _ = std::fs::remove_file(path);
            return Err(error.into());
        }
    };
    snapshot_guard.preservar();
    if automatico {
        limpar_antigos(state).await?;
    }
    Ok(registro)
}

async fn carregar_configuracao(pool: &SqlitePool) -> Result<BackupConfiguracao, AppError> {
    Ok(sqlx::query_as(
        "SELECT ativo, horario, manter_diarios, manter_semanais, manter_mensais, ultima_execucao_em, ultima_tentativa_em, ultimo_erro \
         FROM backup_configuracao WHERE id = 1",
    )
    .fetch_one(pool)
    .await?)
}

async fn limpar_antigos(state: &AppState) -> Result<(), AppError> {
    let config = carregar_configuracao(&state.pool).await?;
    let registros: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, nome_arquivo, data_criacao FROM backup_registro \
         WHERE automatico = 1 ORDER BY data_criacao DESC, id DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut dias = HashSet::new();
    let mut semanas = HashSet::new();
    let mut meses = HashSet::new();
    let mut manter = HashSet::new();
    for (id, _, data) in &registros {
        let Some(data) = data_backup_local(data) else {
            manter.insert(*id);
            continue;
        };
        let dia = data.date_naive();
        let iso = dia.iso_week();
        let semana = (iso.year(), iso.week());
        let mes = (dia.year(), dia.month());
        if dias.len() < config.manter_diarios as usize && dias.insert(dia) {
            manter.insert(*id);
        }
        if semanas.len() < config.manter_semanais as usize && semanas.insert(semana) {
            manter.insert(*id);
        }
        if meses.len() < config.manter_mensais as usize && meses.insert(mes) {
            manter.insert(*id);
        }
    }

    let dir = diretorio_backup(state)?;
    for (id, nome, _) in registros {
        if manter.contains(&id) {
            continue;
        }
        let path = dir.join(&nome);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        sqlx::query("DELETE FROM backup_registro WHERE id = ?")
            .bind(id)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

fn data_backup_local(valor: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(valor)
        .ok()
        .map(|data| data.with_timezone(&Local))
        .or_else(|| {
            NaiveDateTime::parse_from_str(valor, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|data| Utc.from_utc_datetime(&data).with_timezone(&Local))
        })
}

pub fn iniciar_rotina(state: AppState) {
    limpar_temporarios(&state);
    tokio::spawn(async move {
        loop {
            limpar_pendentes_expirados(&state).await;
            if let Err(error) = executar_backup_agendado(&state).await {
                tracing::error!(%error, "backup automático falhou");
            }
            tokio::time::sleep(StdDuration::from_secs(60)).await;
        }
    });
}

async fn executar_backup_agendado(state: &AppState) -> Result<(), AppError> {
    let config = carregar_configuracao(&state.pool).await?;
    if !config.ativo {
        return Ok(());
    }
    let horario = NaiveTime::parse_from_str(&config.horario, "%H:%M")
        .map_err(|_| AppError::interno("horário de backup inválido"))?;
    let agora = Local::now();
    let tentado_hoje = config
        .ultima_tentativa_em
        .as_deref()
        .and_then(data_backup_local)
        .is_some_and(|ultima| ultima.date_naive() == agora.date_naive());
    if agora.time() < horario || tentado_hoje {
        return Ok(());
    }
    let Ok(_operacao) = state.backup_runtime.operacao.try_lock() else {
        return Ok(());
    };
    let agora_utc = Utc::now().to_rfc3339();
    match criar_snapshot_interno(state, "automatico").await {
        Ok(_) => {
            sqlx::query(
                "UPDATE backup_configuracao SET ultima_execucao_em = ?, ultima_tentativa_em = ?, \
                 ultimo_erro = NULL, data_atualizacao = CURRENT_TIMESTAMP WHERE id = 1",
            )
            .bind(&agora_utc)
            .bind(&agora_utc)
            .execute(&state.pool)
            .await?;
            Ok(())
        }
        Err(error) => {
            let _ = sqlx::query(
                "UPDATE backup_configuracao SET ultima_tentativa_em = ?, ultimo_erro = ?, \
                 data_atualizacao = CURRENT_TIMESTAMP WHERE id = 1",
            )
            .bind(&agora_utc)
            .bind(error.to_string())
            .execute(&state.pool)
            .await;
            Err(error)
        }
    }
}

async fn listar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<Vec<BackupInfo>>, AppError> {
    exigir_admin(&sessao)?;
    Ok(Json(sqlx::query_as(
        "SELECT id, nome_arquivo, tamanho_bytes, automatico, data_criacao, tipo, sha256, integridade_ok, versao_app, schema_versao \
         FROM backup_registro ORDER BY id DESC LIMIT 100",
    ).fetch_all(&state.pool).await?))
}

async fn obter_configuracao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_admin(&sessao)?;
    let config = carregar_configuracao(&state.pool).await?;
    Ok(Json(serde_json::json!({
        "ativo": config.ativo,
        "horario": config.horario,
        "manter_diarios": config.manter_diarios,
        "manter_semanais": config.manter_semanais,
        "manter_mensais": config.manter_mensais,
        "ultima_execucao_em": config.ultima_execucao_em,
        "ultima_tentativa_em": config.ultima_tentativa_em,
        "ultimo_erro": config.ultimo_erro,
        "proxima_execucao_em": proxima_execucao(&config),
        "max_upload_bytes": state.config.backup_max_upload_bytes,
    })))
}

fn proxima_execucao(config: &BackupConfiguracao) -> Option<String> {
    if !config.ativo {
        return None;
    }
    let horario = NaiveTime::parse_from_str(&config.horario, "%H:%M").ok()?;
    let agora = Local::now();
    let tentado_hoje = config
        .ultima_tentativa_em
        .as_deref()
        .and_then(data_backup_local)
        .is_some_and(|ultima| ultima.date_naive() == agora.date_naive());
    let data = if !tentado_hoje && agora.time() < horario {
        agora.date_naive()
    } else {
        agora.date_naive().succ_opt()?
    };
    Local
        .from_local_datetime(&data.and_time(horario))
        .earliest()
        .map(|data| data.to_rfc3339())
}

async fn atualizar_configuracao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<BackupConfiguracao>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_admin(&sessao)?;
    let _operacao = state
        .backup_runtime
        .operacao
        .try_lock()
        .map_err(|_| AppError::Conflict("há uma operação de backup em andamento".into()))?;
    NaiveTime::parse_from_str(&input.horario, "%H:%M")
        .map_err(|_| AppError::BadRequest("informe um horário válido".into()))?;
    if !(0..=365).contains(&input.manter_diarios)
        || !(0..=104).contains(&input.manter_semanais)
        || !(0..=60).contains(&input.manter_mensais)
    {
        return Err(AppError::BadRequest(
            "a retenção deve respeitar os limites exibidos".into(),
        ));
    }
    if input.ativo
        && input.manter_diarios == 0
        && input.manter_semanais == 0
        && input.manter_mensais == 0
    {
        return Err(AppError::BadRequest(
            "mantenha ao menos uma cópia automática".into(),
        ));
    }
    sqlx::query(
        "UPDATE backup_configuracao SET ativo = ?, horario = ?, manter_diarios = ?, \
         manter_semanais = ?, manter_mensais = ?, data_atualizacao = CURRENT_TIMESTAMP WHERE id = 1",
    )
    .bind(input.ativo)
    .bind(&input.horario)
    .bind(input.manter_diarios)
    .bind(input.manter_semanais)
    .bind(input.manter_mensais)
    .execute(&state.pool)
    .await?;
    limpar_antigos(&state).await?;
    drop(_operacao);
    obter_configuracao(State(state), Extension(sessao)).await
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
    let info: BackupInfo = sqlx::query_as(
        "SELECT id, nome_arquivo, tamanho_bytes, automatico, data_criacao, tipo, sha256, integridade_ok, versao_app, schema_versao \
         FROM backup_registro WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::nao_encontrado("backup"))?;
    responder_arquivo(
        diretorio_backup(&state)?.join(&info.nome_arquivo),
        &info.nome_arquivo,
        "application/vnd.sqlite3",
        false,
    )
    .await
}

async fn excluir(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    exigir_admin(&sessao)?;
    let _operacao = state
        .backup_runtime
        .operacao
        .try_lock()
        .map_err(|_| AppError::Conflict("há uma operação de backup em andamento".into()))?;
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
) -> Result<(StatusCode, Json<ExportacaoPronta>), AppError> {
    exigir_admin(&sessao)?;
    if input.senha.chars().count() < 10 || input.senha.chars().count() > 1024 {
        return Err(AppError::BadRequest(
            "use uma senha de exportação entre 10 e 1024 caracteres".into(),
        ));
    }
    let _operacao =
        state.backup_runtime.operacao.try_lock().map_err(|_| {
            AppError::Conflict("já existe uma operação de backup em andamento".into())
        })?;
    let backup = criar_snapshot_interno(&state, "manual").await?;
    let dir = diretorio_backup(&state)?;
    let db_path = dir.join(&backup.nome_arquivo);
    conferir_espaco(
        &dir,
        u64::try_from(backup.tamanho_bytes).unwrap_or(u64::MAX),
    )?;
    let zip_path = dir.join(format!(".exportacao-{}.zip", Uuid::new_v4().simple()));
    let db_validacao = db_path.clone();
    let validacao = tokio::task::spawn_blocking(move || validar_banco(&db_validacao))
        .await
        .map_err(|_| AppError::interno("falha ao validar a exportação"))??;
    let manifesto = BackupManifesto {
        formato: FORMATO_BACKUP,
        aplicacao: "AgendarX".into(),
        versao: env!("CARGO_PKG_VERSION").into(),
        criado_em: Utc::now().to_rfc3339(),
        tamanho_banco: u64::try_from(backup.tamanho_bytes).unwrap_or_default(),
        sha256: backup.sha256.clone(),
        schema_versao: validacao.schema_versao,
        pessoas: validacao.pessoas,
        usuarios: validacao.usuarios,
        anexos: validacao.anexos,
    };
    let senha = input.senha;
    let destino = zip_path.clone();
    let resultado = tokio::task::spawn_blocking(move || {
        criar_zip_criptografado(&db_path, &destino, &senha, &manifesto)
    })
    .await;
    let resultado = match resultado {
        Ok(resultado) => resultado,
        Err(_) => {
            let _ = std::fs::remove_file(&zip_path);
            return Err(AppError::interno("falha ao gerar a exportação"));
        }
    };
    if let Err(error) = resultado {
        let _ = std::fs::remove_file(&zip_path);
        return Err(error);
    }
    let token = Uuid::new_v4().simple().to_string();
    let nome = format!("agendarx-backup-{}.zip", Utc::now().format("%Y%m%d-%H%M%S"));
    let tamanho = std::fs::metadata(&zip_path)?.len();
    let expira_em = Utc::now() + chrono::Duration::minutes(10);
    let mut exportacoes = state.backup_runtime.exportacoes.lock().await;
    let anteriores: Vec<String> = exportacoes
        .iter()
        .filter(|(_, item)| item.usuario_id == sessao.usuario.id)
        .map(|(token, _)| token.clone())
        .collect();
    for anterior in anteriores {
        if let Some(item) = exportacoes.remove(&anterior) {
            let _ = std::fs::remove_file(item.caminho);
        }
    }
    exportacoes.insert(
        token.clone(),
        ExportacaoPendente {
            caminho: zip_path,
            nome: nome.clone(),
            usuario_id: sessao.usuario.id,
            expira_em,
        },
    );
    drop(exportacoes);
    Ok((
        StatusCode::CREATED,
        Json(ExportacaoPronta {
            token,
            nome_arquivo: nome,
            tamanho_bytes: tamanho,
            expira_em: expira_em.to_rfc3339(),
        }),
    ))
}

async fn baixar_exportacao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(token): Path<String>,
) -> Result<Response, AppError> {
    exigir_admin(&sessao)?;
    let item = {
        let mut exportacoes = state.backup_runtime.exportacoes.lock().await;
        let item = exportacoes
            .get(&token)
            .ok_or_else(|| AppError::nao_encontrado("exportação"))?;
        if item.usuario_id != sessao.usuario.id {
            return Err(AppError::Forbidden);
        }
        if item.expira_em <= Utc::now() {
            let expirado = exportacoes.remove(&token).expect("item verificado");
            let _ = std::fs::remove_file(expirado.caminho);
            return Err(AppError::BadRequest(
                "a exportação expirou; gere um novo backup".into(),
            ));
        }
        exportacoes.remove(&token).expect("item verificado")
    };
    responder_arquivo(item.caminho, &item.nome, "application/zip", true).await
}

fn criar_zip_criptografado(
    db_path: &FsPath,
    zip_path: &FsPath,
    senha: &str,
    manifesto: &BackupManifesto,
) -> Result<(), AppError> {
    let destino = File::create(zip_path)?;
    restringir_permissoes(zip_path)?;
    let mut writer = ZipWriter::new(destino);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .with_aes_encryption(AesMode::Aes256, senha);
    writer
        .start_file("agendarx.db", options)
        .map_err(|error| AppError::interno(error.to_string()))?;
    let mut origem = BufReader::new(File::open(db_path)?);
    std::io::copy(&mut origem, &mut writer)?;
    writer
        .start_file("manifest.json", options)
        .map_err(|error| AppError::interno(error.to_string()))?;
    serde_json::to_writer_pretty(&mut writer, manifesto)
        .map_err(|error| AppError::interno(error.to_string()))?;
    writer
        .finish()
        .map_err(|error| AppError::interno(error.to_string()))?;
    Ok(())
}

async fn preparar_restauracao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    mut multipart: Multipart,
) -> Result<Json<RestauracaoPrevia>, AppError> {
    exigir_admin(&sessao)?;
    let _operacao = state
        .backup_runtime
        .operacao
        .try_lock()
        .map_err(|_| AppError::Conflict("há uma operação de backup em andamento".into()))?;
    limpar_pendentes_expirados(&state).await;
    let dir = diretorio_backup(&state)?;
    std::fs::create_dir_all(&dir)?;
    let upload_path = dir.join(format!(".upload-{}", Uuid::new_v4().simple()));
    let mut upload_guard = ArquivoTemporario::new(upload_path.clone());
    let mut recebeu_arquivo = false;
    let mut senha = String::new();
    while let Some(mut field) = multipart.next_field().await.map_err(AppError::from)? {
        match field.name() {
            Some("arquivo") if !recebeu_arquivo => {
                let mut destino = tokio::fs::File::create(&upload_path).await?;
                restringir_permissoes(&upload_path)?;
                let mut tamanho = 0usize;
                while let Some(chunk) = field.chunk().await.map_err(AppError::from)? {
                    tamanho = tamanho.saturating_add(chunk.len());
                    if tamanho > state.config.backup_max_upload_bytes {
                        return Err(AppError::PayloadTooLarge);
                    }
                    destino.write_all(&chunk).await?;
                }
                destino.flush().await?;
                recebeu_arquivo = true;
            }
            Some("senha") => {
                senha = field.text().await.map_err(AppError::from)?;
                if senha.chars().count() > 1024 {
                    return Err(AppError::BadRequest("senha longa demais".into()));
                }
            }
            _ => {}
        }
    }
    if !recebeu_arquivo {
        return Err(AppError::BadRequest("selecione um backup".into()));
    }
    let token = Uuid::new_v4().simple().to_string();
    let db_path = dir.join(format!(".restauracao-{token}.db"));
    let mut db_guard = ArquivoTemporario::new(db_path.clone());
    let origem = upload_path.clone();
    let destino = db_path.clone();
    let max = state.config.backup_max_upload_bytes;
    let preparado =
        tokio::task::spawn_blocking(move || preparar_arquivo(&origem, &destino, &senha, max))
            .await
            .map_err(|_| AppError::interno("falha ao preparar a restauração"))?;
    let (validacao, manifesto, criptografado, sha256) = match preparado {
        Ok(resultado) => resultado,
        Err(error) => {
            let _ = std::fs::remove_file(&db_path);
            return Err(error);
        }
    };
    upload_guard.remover_agora();
    let tamanho_bytes = std::fs::metadata(&db_path)?.len();
    let expira_em = Utc::now() + chrono::Duration::minutes(EXPIRACAO_RESTORE_MINUTOS);
    let mut avisos = Vec::new();
    if manifesto.is_none() {
        avisos
            .push("Backup SQLite sem manifesto; origem e data não puderam ser confirmadas.".into());
    }
    if validacao.schema_versao < SCHEMA_ATUAL {
        avisos.push("O backup é de uma versão anterior e será migrado automaticamente.".into());
    }
    let previa = RestauracaoPrevia {
        token: token.clone(),
        expira_em: expira_em.to_rfc3339(),
        criado_em: manifesto.as_ref().map(|item| item.criado_em.clone()),
        versao_app: manifesto.as_ref().map(|item| item.versao.clone()),
        schema_versao: validacao.schema_versao,
        schema_atual: SCHEMA_ATUAL,
        tamanho_bytes,
        sha256,
        pessoas: validacao.pessoas,
        usuarios: validacao.usuarios,
        anexos: validacao.anexos,
        criptografado,
        avisos,
    };
    let mut pendentes = state.backup_runtime.pendentes.lock().await;
    let anteriores: Vec<String> = pendentes
        .iter()
        .filter(|(_, item)| item.usuario_id == sessao.usuario.id)
        .map(|(token, _)| token.clone())
        .collect();
    for anterior in anteriores {
        if let Some(item) = pendentes.remove(&anterior) {
            let _ = std::fs::remove_file(item.caminho);
        }
    }
    pendentes.insert(
        token,
        RestauracaoPendente {
            caminho: db_path,
            usuario_id: sessao.usuario.id,
            expira_em,
            previa: previa.clone(),
        },
    );
    drop(pendentes);
    db_guard.preservar();
    Ok(Json(previa))
}

fn preparar_arquivo(
    origem: &FsPath,
    destino: &FsPath,
    senha: &str,
    max_bytes: usize,
) -> Result<(ValidacaoBanco, Option<BackupManifesto>, bool, String), AppError> {
    let mut assinatura = [0u8; 16];
    let mut arquivo = File::open(origem)?;
    let lidos = arquivo.read(&mut assinatura)?;
    drop(arquivo);
    let mut manifesto: Option<BackupManifesto> = None;
    let criptografado;
    if lidos == assinatura.len() && &assinatura == b"SQLite format 3\0" {
        if std::fs::metadata(origem)?.len() > max_bytes as u64 {
            return Err(AppError::PayloadTooLarge);
        }
        std::fs::copy(origem, destino)?;
        restringir_permissoes(destino)?;
        criptografado = false;
    } else {
        let arquivo = File::open(origem)?;
        let mut zip = ZipArchive::new(arquivo).map_err(|_| {
            AppError::BadRequest("o arquivo não é um backup SQLite ou ZIP válido".into())
        })?;
        let db_index = (0..zip.len())
            .find(|indice| zip.file_names().nth(*indice) == Some("agendarx.db"))
            .ok_or_else(|| AppError::BadRequest("o ZIP não contém agendarx.db".into()))?;
        let zip_criptografado;
        {
            let banco = if senha.is_empty() {
                zip.by_index(db_index)
            } else {
                zip.by_index_decrypt(db_index, senha.as_bytes())
            }
            .map_err(|_| AppError::BadRequest("senha incorreta ou ZIP inválido".into()))?;
            zip_criptografado = banco.encrypted();
            if banco.size() > max_bytes as u64 {
                return Err(AppError::PayloadTooLarge);
            }
            let mut saida = File::create(destino)?;
            restringir_permissoes(destino)?;
            let copiados = std::io::copy(&mut banco.take(max_bytes as u64 + 1), &mut saida)
                .map_err(|_| AppError::BadRequest("não foi possível extrair o backup".into()))?;
            if copiados > max_bytes as u64 {
                let _ = std::fs::remove_file(destino);
                return Err(AppError::PayloadTooLarge);
            }
        }
        if let Some(index) =
            (0..zip.len()).find(|indice| zip.file_names().nth(*indice) == Some("manifest.json"))
        {
            let mut item = if senha.is_empty() {
                zip.by_index(index)
            } else {
                zip.by_index_decrypt(index, senha.as_bytes())
            }
            .map_err(|_| AppError::BadRequest("senha incorreta ou manifesto inválido".into()))?;
            if item.size() > 64 * 1024 {
                return Err(AppError::BadRequest(
                    "manifesto do backup é grande demais".into(),
                ));
            }
            manifesto = Some(
                serde_json::from_reader(&mut item)
                    .map_err(|_| AppError::BadRequest("manifesto do backup inválido".into()))?,
            );
        }
        criptografado = zip_criptografado;
    }

    let hash = hash_arquivo(destino)?;
    if let Some(item) = &manifesto {
        if item.formato != FORMATO_BACKUP || item.aplicacao != "AgendarX" {
            return Err(AppError::BadRequest(
                "formato de backup incompatível".into(),
            ));
        }
        if item.sha256 != hash || item.tamanho_banco != std::fs::metadata(destino)?.len() {
            return Err(AppError::BadRequest(
                "o backup está incompleto ou foi corrompido".into(),
            ));
        }
    }
    let validacao = validar_banco(destino)?;
    if let Some(item) = &manifesto {
        if item.schema_versao != validacao.schema_versao {
            return Err(AppError::BadRequest(
                "o manifesto não corresponde ao banco enviado".into(),
            ));
        }
        if item.pessoas != validacao.pessoas
            || item.usuarios != validacao.usuarios
            || item.anexos != validacao.anexos
        {
            return Err(AppError::BadRequest(
                "os totais do manifesto não correspondem ao banco enviado".into(),
            ));
        }
    }
    Ok((validacao, manifesto, criptografado, hash))
}

async fn confirmar_restauracao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(token): Path<String>,
    Json(input): Json<ConfirmarInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    exigir_admin(&sessao)?;
    if input.confirmacao.trim() != "RESTAURAR" {
        return Err(AppError::BadRequest(
            "digite RESTAURAR para confirmar a substituição".into(),
        ));
    }
    let _operacao =
        state.backup_runtime.operacao.try_lock().map_err(|_| {
            AppError::Conflict("já existe uma operação de backup em andamento".into())
        })?;
    let pendente = {
        let mut pendentes = state.backup_runtime.pendentes.lock().await;
        let item = pendentes
            .get(&token)
            .ok_or_else(|| AppError::nao_encontrado("restauração preparada"))?;
        if item.usuario_id != sessao.usuario.id {
            return Err(AppError::Forbidden);
        }
        if item.expira_em <= Utc::now() {
            let expirado = pendentes.remove(&token).expect("item verificado");
            let _ = std::fs::remove_file(expirado.caminho);
            return Err(AppError::BadRequest(
                "a prévia expirou; envie o backup novamente".into(),
            ));
        }
        pendentes.remove(&token).expect("item verificado")
    };
    let _arquivo_pendente = ArquivoTemporario::new(pendente.caminho.clone());
    let _manutencao = state.backup_runtime.manutencao.write().await;
    let seguranca = criar_snapshot_interno(&state, "seguranca").await?;
    let catalogo = listar_catalogo(&state.pool).await?;
    let caminho_atual = caminho_banco(&state)?;
    let caminho_novo = pendente.caminho.clone();
    let atual = caminho_atual.clone();
    let copia = tokio::task::spawn_blocking(move || copiar_banco(&caminho_novo, &atual))
        .await
        .map_err(|_| AppError::interno("falha ao executar a restauração"))?;
    if let Err(error) = copia {
        recuperar_seguranca(&state, &seguranca, &catalogo, &caminho_atual).await?;
        return Err(AppError::interno(format!(
            "a restauração foi interrompida e o estado anterior foi recuperado: {error}"
        )));
    }

    let pos_restore =
        finalizar_restauracao(&state, &catalogo, &sessao.usuario.login, &pendente.previa).await;
    if let Err(error) = pos_restore {
        tracing::error!(%error, "validação posterior falhou; recuperando backup de segurança");
        recuperar_seguranca(&state, &seguranca, &catalogo, &caminho_atual).await?;
        return Err(AppError::interno(format!(
            "a restauração falhou e o estado anterior foi recuperado: {error}"
        )));
    }
    Ok(Json(serde_json::json!({
        "mensagem": "backup completo restaurado",
        "backup_seguranca": seguranca.nome_arquivo,
        "sessao_encerrada": true,
    })))
}

async fn recuperar_seguranca(
    state: &AppState,
    seguranca: &BackupInfo,
    catalogo: &[BackupInfo],
    caminho_atual: &FsPath,
) -> Result<(), AppError> {
    let seguranca_path = diretorio_backup(state)?.join(&seguranca.nome_arquivo);
    let atual = caminho_atual.to_path_buf();
    tokio::task::spawn_blocking(move || copiar_banco(&seguranca_path, &atual))
        .await
        .map_err(|_| AppError::interno("falha ao recuperar o backup de segurança"))??;
    sqlx::migrate!("./migrations").run(&state.pool).await?;
    reconciliar_catalogo(state, catalogo).await?;
    Ok(())
}

async fn finalizar_restauracao(
    state: &AppState,
    catalogo: &[BackupInfo],
    usuario_login: &str,
    previa: &RestauracaoPrevia,
) -> Result<(), AppError> {
    sqlx::migrate!("./migrations").run(&state.pool).await?;
    sqlx::query("DELETE FROM sessao")
        .execute(&state.pool)
        .await?;
    reconciliar_catalogo(state, catalogo).await?;
    let caminho = caminho_banco(state)?;
    let validacao = tokio::task::spawn_blocking(move || validar_banco(&caminho))
        .await
        .map_err(|_| AppError::interno("falha ao validar o banco restaurado"))??;
    if validacao.usuarios == 0 {
        return Err(AppError::BadRequest(
            "o backup restaurado não possui usuários".into(),
        ));
    }
    sqlx::query(
        "INSERT INTO auditoria (usuario_id, usuario_login, acao, recurso, status_http) \
         VALUES (NULL, ?, 'RESTAURAR_BACKUP', ?, 200)",
    )
    .bind(usuario_login)
    .bind(format!("sha256:{}", previa.sha256))
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn copiar_banco(origem: &FsPath, destino: &FsPath) -> Result<(), AppError> {
    let mut conexao = Connection::open(destino)
        .map_err(|error| AppError::interno(format!("falha ao abrir o banco atual: {error}")))?;
    conexao
        .busy_timeout(StdDuration::from_secs(10))
        .map_err(|error| AppError::interno(error.to_string()))?;
    conexao
        .restore(
            DatabaseName::Main,
            origem,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|error| AppError::interno(format!("falha ao copiar o banco: {error}")))?;
    Ok(())
}

async fn cancelar_restauracao(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(token): Path<String>,
) -> Result<StatusCode, AppError> {
    exigir_admin(&sessao)?;
    let mut pendentes = state.backup_runtime.pendentes.lock().await;
    let item = pendentes
        .get(&token)
        .ok_or_else(|| AppError::nao_encontrado("restauração preparada"))?;
    if item.usuario_id != sessao.usuario.id {
        return Err(AppError::Forbidden);
    }
    let item = pendentes.remove(&token).expect("item verificado");
    let _ = std::fs::remove_file(item.caminho);
    Ok(StatusCode::NO_CONTENT)
}

async fn limpar_pendentes_expirados(state: &AppState) {
    let agora = Utc::now();
    let mut pendentes = state.backup_runtime.pendentes.lock().await;
    let expirados: Vec<String> = pendentes
        .iter()
        .filter(|(_, item)| item.expira_em <= agora)
        .map(|(token, _)| token.clone())
        .collect();
    for token in expirados {
        if let Some(item) = pendentes.remove(&token) {
            let _ = std::fs::remove_file(item.caminho);
        }
    }
    drop(pendentes);

    let mut exportacoes = state.backup_runtime.exportacoes.lock().await;
    let expiradas: Vec<String> = exportacoes
        .iter()
        .filter(|(_, item)| item.expira_em <= agora)
        .map(|(token, _)| token.clone())
        .collect();
    for token in expiradas {
        if let Some(item) = exportacoes.remove(&token) {
            let _ = std::fs::remove_file(item.caminho);
        }
    }
}

fn limpar_temporarios(state: &AppState) {
    let Ok(dir) = diretorio_backup(state) else {
        return;
    };
    let Ok(items) = std::fs::read_dir(dir) else {
        return;
    };
    for item in items.flatten() {
        let nome = item.file_name();
        let nome = nome.to_string_lossy();
        if nome.starts_with(".upload-")
            || nome.starts_with(".restauracao-")
            || nome.starts_with(".exportacao-")
        {
            let _ = std::fs::remove_file(item.path());
        }
    }
}

async fn listar_catalogo(pool: &SqlitePool) -> Result<Vec<BackupInfo>, AppError> {
    Ok(sqlx::query_as(
        "SELECT id, nome_arquivo, tamanho_bytes, automatico, data_criacao, tipo, sha256, integridade_ok, versao_app, schema_versao FROM backup_registro ORDER BY id",
    )
    .fetch_all(pool)
    .await?)
}

async fn reconciliar_catalogo(state: &AppState, catalogo: &[BackupInfo]) -> Result<(), AppError> {
    sqlx::query("DELETE FROM backup_registro")
        .execute(&state.pool)
        .await?;
    let dir = diretorio_backup(state)?;
    for item in catalogo {
        if !dir.join(&item.nome_arquivo).is_file() {
            continue;
        }
        sqlx::query(
            "INSERT INTO backup_registro (nome_arquivo, tamanho_bytes, automatico, data_criacao, tipo, sha256, integridade_ok, versao_app, schema_versao) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&item.nome_arquivo)
        .bind(item.tamanho_bytes)
        .bind(item.automatico)
        .bind(&item.data_criacao)
        .bind(&item.tipo)
        .bind(&item.sha256)
        .bind(item.integridade_ok)
        .bind(&item.versao_app)
        .bind(item.schema_versao)
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

fn validar_banco(caminho: &FsPath) -> Result<ValidacaoBanco, AppError> {
    let conexao = Connection::open_with_flags(
        caminho,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| AppError::BadRequest("não foi possível abrir o banco SQLite".into()))?;
    let integridade: String = conexao
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| AppError::BadRequest("não foi possível verificar o banco".into()))?;
    if integridade != "ok" {
        return Err(AppError::BadRequest(format!(
            "o banco está corrompido: {integridade}"
        )));
    }
    let required: i64 = conexao
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('usuario','pessoa','anexo_dossie')",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    if required != 3 {
        return Err(AppError::BadRequest(
            "backup incompatível: faltam tabelas essenciais do AgendarX".into(),
        ));
    }
    let mut foreign_keys = conexao
        .prepare("PRAGMA foreign_key_check")
        .map_err(|_| AppError::BadRequest("não foi possível verificar relacionamentos".into()))?;
    if foreign_keys
        .exists([])
        .map_err(|_| AppError::BadRequest("não foi possível verificar relacionamentos".into()))?
    {
        return Err(AppError::BadRequest(
            "o backup possui relacionamentos inválidos".into(),
        ));
    }
    let schema_versao = conexao
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or_default();
    if schema_versao > SCHEMA_ATUAL {
        return Err(AppError::BadRequest(format!(
            "backup criado por uma versão mais nova do AgendarX (schema {schema_versao})"
        )));
    }
    if schema_versao == 0 {
        return Err(AppError::BadRequest(
            "backup sem histórico de migrações do AgendarX".into(),
        ));
    }
    let pessoas = contar_tabela(&conexao, "pessoa")?;
    let usuarios = contar_tabela(&conexao, "usuario")?;
    if usuarios == 0 {
        return Err(AppError::BadRequest(
            "o backup não possui uma conta para login".into(),
        ));
    }
    let administradores = if schema_versao >= 12 {
        conexao
            .query_row(
                "SELECT COUNT(*) FROM usuario WHERE perfil = 'admin'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or_default()
    } else {
        usuarios
    };
    if administradores == 0 {
        return Err(AppError::BadRequest(
            "o backup não possui uma conta de administrador".into(),
        ));
    }
    let anexos = ["anexo_dossie", "anexo_vinculo", "anexo_tarefa_calendario"]
        .iter()
        .map(|tabela| contar_tabela_se_existir(&conexao, tabela))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum();
    Ok(ValidacaoBanco {
        schema_versao,
        pessoas,
        usuarios,
        anexos,
    })
}

fn contar_tabela(conexao: &Connection, tabela: &str) -> Result<i64, AppError> {
    conexao
        .query_row(&format!("SELECT COUNT(*) FROM {tabela}"), [], |row| {
            row.get(0)
        })
        .map_err(|_| AppError::BadRequest(format!("não foi possível ler a tabela {tabela}")))
}

fn contar_tabela_se_existir(conexao: &Connection, tabela: &str) -> Result<i64, AppError> {
    let existe: bool = conexao
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?)",
            [tabela],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if existe {
        contar_tabela(conexao, tabela)
    } else {
        Ok(0)
    }
}

fn hash_arquivo(caminho: &FsPath) -> Result<String, AppError> {
    let mut arquivo = BufReader::new(File::open(caminho)?);
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let lidos = arquivo.read(&mut buffer)?;
        if lidos == 0 {
            break;
        }
        hash.update(&buffer[..lidos]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn restringir_permissoes(caminho: &FsPath) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(caminho, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

struct LeitorArquivo {
    arquivo: tokio::fs::File,
    remover: Option<PathBuf>,
}

impl AsyncRead for LeitorArquivo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.arquivo).poll_read(cx, buf)
    }
}

impl Drop for LeitorArquivo {
    fn drop(&mut self) {
        if let Some(caminho) = self.remover.take() {
            let _ = std::fs::remove_file(caminho);
        }
    }
}

struct ArquivoTemporario {
    caminho: Option<PathBuf>,
}

impl ArquivoTemporario {
    fn new(caminho: PathBuf) -> Self {
        Self {
            caminho: Some(caminho),
        }
    }

    fn remover_agora(&mut self) {
        if let Some(caminho) = self.caminho.take() {
            let _ = std::fs::remove_file(caminho);
        }
    }

    fn preservar(&mut self) {
        self.caminho.take();
    }
}

impl Drop for ArquivoTemporario {
    fn drop(&mut self) {
        self.remover_agora();
    }
}

async fn responder_arquivo(
    caminho: PathBuf,
    nome: &str,
    mime: &str,
    remover: bool,
) -> Result<Response, AppError> {
    let mut guard = remover.then(|| ArquivoTemporario::new(caminho.clone()));
    let tamanho = tokio::fs::metadata(&caminho).await?.len();
    let arquivo = tokio::fs::File::open(&caminho).await?;
    let leitor = LeitorArquivo {
        arquivo,
        remover: remover.then_some(caminho),
    };
    if let Some(guard) = &mut guard {
        guard.preservar();
    }
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{nome}\""),
        )
        .header(header::CONTENT_LENGTH, tamanho)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from_stream(ReaderStream::new(leitor)))
        .map_err(|error| AppError::interno(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        BackupManifesto, FORMATO_BACKUP, copiar_banco, criar_zip_criptografado, hash_arquivo,
        preparar_arquivo, validar_banco,
    };
    use crate::{AppState, config::Config, db, handlers::backup::BackupRuntime};

    #[tokio::test]
    async fn snapshot_validado_preserva_todo_o_banco() {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let mut config = Config::from_env().unwrap();
        config.database_url = format!("sqlite://.cache/backup-test-{id}.db?mode=rwc");
        config.admin_login = Some("admin-backup".into());
        config.admin_password = Some("senha-backup-segura".into());
        let pool = db::conectar(&config).await.unwrap();
        let state = AppState {
            pool: pool.clone(),
            config,
            backup_runtime: BackupRuntime::default(),
        };
        sqlx::query("INSERT INTO pessoa (nome, classificacao_risco, toxicidade) VALUES ('Antes', 'MANIPULATIVO', 0.3)")
            .execute(&pool)
            .await
            .unwrap();
        let parametros_hp = crate::domain::hp_psicossocial::ParametrosHp {
            fator_segundo_grau: 0.4,
            ..Default::default()
        };
        sqlx::query(
            "UPDATE hp_psicossocial_configuracao SET versao = 2, parametros_json = ? WHERE id = 1",
        )
        .bind(serde_json::to_string(&parametros_hp).unwrap())
        .execute(&pool)
        .await
        .unwrap();
        let backup = super::criar_snapshot(&state, false).await.unwrap();
        let path = super::diretorio_backup(&state)
            .unwrap()
            .join(&backup.nome_arquivo);
        let validacao = validar_banco(&path).unwrap();
        assert_eq!(validacao.pessoas, 1);
        assert_eq!(validacao.usuarios, 1);
        assert_eq!(backup.sha256, hash_arquivo(&path).unwrap());
        let zip_path = std::path::PathBuf::from(format!(".cache/backup-{id}.zip"));
        let extraido = std::path::PathBuf::from(format!(".cache/extraido-{id}.db"));
        let manifesto = BackupManifesto {
            formato: FORMATO_BACKUP,
            aplicacao: "AgendarX".into(),
            versao: "teste".into(),
            criado_em: "2026-01-01T00:00:00Z".into(),
            tamanho_banco: std::fs::metadata(&path).unwrap().len(),
            sha256: backup.sha256.clone(),
            schema_versao: validacao.schema_versao,
            pessoas: validacao.pessoas,
            usuarios: validacao.usuarios,
            anexos: validacao.anexos,
        };
        criar_zip_criptografado(&path, &zip_path, "senha-backup-segura", &manifesto).unwrap();
        let (extraido_validacao, extraido_manifesto, criptografado, extraido_hash) =
            preparar_arquivo(
                &zip_path,
                &extraido,
                "senha-backup-segura",
                100 * 1024 * 1024,
            )
            .unwrap();
        assert!(criptografado);
        assert_eq!(extraido_validacao.pessoas, 1);
        assert_eq!(extraido_manifesto.unwrap().formato, FORMATO_BACKUP);
        assert_eq!(extraido_hash, backup.sha256);
        sqlx::query("UPDATE pessoa SET nome = 'Depois', toxicidade = 0.1")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE hp_psicossocial_configuracao SET versao = 99 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        copiar_banco(&path, &super::caminho_banco(&state).unwrap()).unwrap();
        let nome: String = sqlx::query_scalar("SELECT nome FROM pessoa")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(nome, "Antes");
        let risco: (String, f64) =
            sqlx::query_as("SELECT classificacao_risco, toxicidade FROM pessoa")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(risco, ("MANIPULATIVO".into(), 0.3));
        let hp: (i64, String) = sqlx::query_as(
            "SELECT versao, parametros_json FROM hp_psicossocial_configuracao WHERE id = 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(hp.0, 2);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&hp.1).unwrap()["fator_segundo_grau"],
            0.4
        );
        pool.close().await;
        let _ = std::fs::remove_file(format!(".cache/backup-test-{id}.db"));
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(zip_path);
        let _ = std::fs::remove_file(extraido);
    }

    #[test]
    fn manifesto_serializa_formato_atual() {
        let manifesto = BackupManifesto {
            formato: FORMATO_BACKUP,
            aplicacao: "AgendarX".into(),
            versao: "teste".into(),
            criado_em: "2026-01-01T00:00:00Z".into(),
            tamanho_banco: 1,
            sha256: "abc".into(),
            schema_versao: 15,
            pessoas: 1,
            usuarios: 1,
            anexos: 0,
        };
        assert!(
            serde_json::to_string(&manifesto)
                .unwrap()
                .contains("AgendarX")
        );
    }

    #[test]
    fn schema_atual_acompanha_a_ultima_migracao() {
        let versao = sqlx::migrate!("./migrations")
            .iter()
            .last()
            .map(|migration| migration.version)
            .unwrap_or_default();
        assert_eq!(versao, super::SCHEMA_ATUAL);
    }

    #[test]
    fn preparar_arquivo_rejeita_conteudo_qualquer() {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let origem = std::path::PathBuf::from(format!(".cache/invalido-{id}"));
        let destino = std::path::PathBuf::from(format!(".cache/restaurado-{id}.db"));
        std::fs::create_dir_all(".cache").unwrap();
        std::fs::write(&origem, b"nao e um banco").unwrap();
        assert!(preparar_arquivo(&origem, &destino, "", 1024).is_err());
        let _ = std::fs::remove_file(origem);
        let _ = std::fs::remove_file(destino);
    }
}
