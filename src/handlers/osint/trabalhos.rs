use axum::{
    Extension, Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::RwLockReadGuard;
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    middleware::auth::SessaoAutenticada,
    models::{ParametroBusca, VarreduraResponse},
};

#[derive(sqlx::FromRow, Serialize)]
pub(super) struct Trabalho {
    id: String,
    pessoa_id: i64,
    estado: String,
    total: i64,
    processados: i64,
    #[serde(skip_serializing)]
    parametros: String,
    #[serde(skip_serializing)]
    resultado: String,
    erro: Option<String>,
    criado_em: String,
    atualizado_em: String,
}
impl Trabalho {
    fn json(&self) -> Result<serde_json::Value, AppError> {
        let mut value = serde_json::to_value(self).map_err(|e| AppError::interno(e.to_string()))?;
        value["resultado"] =
            serde_json::from_str(&self.resultado).map_err(|e| AppError::interno(e.to_string()))?;
        Ok(value)
    }
}
fn vazio() -> VarreduraResponse {
    VarreduraResponse {
        situacao: "concluida".into(),
        parametros_processados: 0,
        parametros_inconclusivos: 0,
        resultados_encontrados: 0,
        novos_achados: 0,
        pdfs_arquivados: 0,
        fontes_indisponiveis: 0,
        avisos: vec![],
    }
}
const CAMPOS: &str =
    "id,pessoa_id,estado,total,processados,parametros,resultado,erro,criado_em,atualizado_em";

pub(super) async fn criar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(pessoa_id): Path<i64>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), AppError> {
    super::garantir_pessoa(&state, pessoa_id).await?;
    let parametros:Vec<ParametroBusca>=sqlx::query_as("SELECT id,pessoa_id,tipo,valor,provider,ativo FROM parametro_busca WHERE pessoa_id=? AND ativo=1 ORDER BY id").bind(pessoa_id).fetch_all(&state.pool).await?;
    if parametros.is_empty() || parametros.len() > super::MAX_PARAMETROS_ATIVOS {
        return Err(AppError::BadRequest(
            "ative de 1 a 50 parâmetros antes da varredura".into(),
        ));
    }
    let id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    admitir_fila(&mut tx, sessao.usuario.id).await?;
    sqlx::query("INSERT INTO osint_trabalho(id,usuario_id,pessoa_id,estado,parametros,total,resultado) VALUES(?,?,?,'fila',?,?,?)")
        .bind(&id).bind(sessao.usuario.id).bind(pessoa_id).bind(serde_json::to_string(&parametros).map_err(|e|AppError::interno(e.to_string()))?).bind(parametros.len() as i64).bind(serde_json::to_string(&vazio()).unwrap()).execute(&mut *tx).await?;
    let trabalho: Trabalho =
        sqlx::query_as(&format!("SELECT {CAMPOS} FROM osint_trabalho WHERE id=?"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok((axum::http::StatusCode::ACCEPTED, Json(trabalho.json()?)))
}
async fn admitir_fila(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    usuario_id: i64,
) -> Result<(), AppError> {
    let pendentes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM osint_trabalho WHERE estado IN ('fila','executando')",
    )
    .fetch_one(&mut **tx)
    .await?;
    if pendentes >= 100 {
        return Err(AppError::ServiceUnavailable(
            "fila de pesquisas cheia; tente novamente mais tarde".into(),
        ));
    }
    let proprios:i64=sqlx::query_scalar("SELECT COUNT(*) FROM osint_trabalho WHERE usuario_id=? AND estado IN ('fila','executando')").bind(usuario_id).fetch_one(&mut **tx).await?;
    if proprios >= 20 {
        return Err(AppError::BadRequest(
            "aguarde as pesquisas pendentes antes de criar outra".into(),
        ));
    }
    Ok(())
}
pub(super) async fn listar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(pessoa_id): Path<i64>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let rows:Vec<Trabalho>=sqlx::query_as(&format!("SELECT {CAMPOS} FROM osint_trabalho WHERE pessoa_id=? AND usuario_id=? ORDER BY criado_em DESC,rowid DESC LIMIT 10")).bind(pessoa_id).bind(sessao.usuario.id).fetch_all(&state.pool).await?;
    Ok(Json(
        rows.iter().map(Trabalho::json).collect::<Result<_, _>>()?,
    ))
}
pub(super) async fn cancelar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, AppError> {
    let r=sqlx::query("UPDATE osint_trabalho SET estado='cancelado',tentativa=NULL,atualizado_em=CURRENT_TIMESTAMP WHERE id=? AND usuario_id=? AND estado IN ('fila','executando')").bind(id).bind(sessao.usuario.id).execute(&state.pool).await?;
    if r.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "trabalho inexistente ou já encerrado".into(),
        ));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}
pub(super) async fn retomar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, AppError> {
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    admitir_fila(&mut tx, sessao.usuario.id).await?;
    let r=sqlx::query("UPDATE osint_trabalho SET estado='fila',erro=NULL,tentativa=NULL,atualizado_em=CURRENT_TIMESTAMP WHERE id=? AND usuario_id=? AND estado IN ('erro','interrompido','cancelado') AND EXISTS(SELECT 1 FROM pessoa WHERE pessoa.id=osint_trabalho.pessoa_id AND excluida_em IS NULL)").bind(id).bind(sessao.usuario.id).execute(&mut *tx).await?;
    if r.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "este trabalho não pode ser retomado".into(),
        ));
    }
    tx.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
pub async fn interromper_trabalhos(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("UPDATE osint_trabalho SET estado='interrompido',tentativa=NULL,erro='Pesquisa interrompida por reinício ou restauração; retome para continuar.',atualizado_em=CURRENT_TIMESTAMP WHERE estado IN ('fila','executando')").execute(pool).await?;
    Ok(())
}
pub async fn iniciar_rotina(state: AppState) -> Result<(), AppError> {
    sqlx::query("UPDATE osint_trabalho SET estado='interrompido',tentativa=NULL,erro='Pesquisa interrompida por reinício; retome para continuar.',atualizado_em=CURRENT_TIMESTAMP WHERE estado='executando'").execute(&state.pool).await?;
    tokio::spawn(async move {
        loop {
            if let Err(error) = proximo(&state).await {
                tracing::error!(%error,"worker de pesquisa pública falhou");
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    Ok(())
}
pub(super) struct Contexto {
    id: String,
    tentativa: String,
    geracao: u64,
}
impl Contexto {
    pub(super) async fn aguardar<T>(
        &self,
        state: &AppState,
        futuro: impl std::future::Future<Output = T>,
    ) -> Result<T, AppError> {
        tokio::pin!(futuro);
        loop {
            tokio::select! {
                result = &mut futuro => return Ok(result),
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => { let _leitura = self.validar(state).await?; }
            }
        }
    }

    pub(super) async fn validar<'a>(
        &self,
        state: &'a AppState,
    ) -> Result<RwLockReadGuard<'a, ()>, AppError> {
        let leitura = state.backup_runtime.leitura().await;
        if state.backup_runtime.geracao() != self.geracao {
            return Err(AppError::Conflict(
                "banco restaurado durante a pesquisa".into(),
            ));
        }
        let ativo:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM osint_trabalho j JOIN pessoa p ON p.id=j.pessoa_id WHERE j.id=? AND j.tentativa=? AND j.estado='executando' AND p.excluida_em IS NULL)").bind(&self.id).bind(&self.tentativa).fetch_one(&state.pool).await?;
        if !ativo {
            return Err(AppError::Conflict("pesquisa interrompida".into()));
        }
        Ok(leitura)
    }
}
async fn proximo(state: &AppState) -> Result<(), AppError> {
    let leitura = state.backup_runtime.leitura().await;
    let geracao = state.backup_runtime.geracao();
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    let trabalho: Option<Trabalho> = sqlx::query_as(&format!(
        "SELECT {CAMPOS} FROM osint_trabalho WHERE estado='fila' ORDER BY criado_em,rowid LIMIT 1"
    ))
    .fetch_optional(&mut *tx)
    .await?;
    let Some(trabalho) = trabalho else {
        return Ok(());
    };
    let tentativa = Uuid::new_v4().to_string();
    sqlx::query("UPDATE osint_trabalho SET estado='executando',tentativa=?,atualizado_em=CURRENT_TIMESTAMP WHERE id=?").bind(&tentativa).bind(&trabalho.id).execute(&mut *tx).await?;
    tx.commit().await?;
    drop(leitura);
    let contexto = Contexto {
        id: trabalho.id.clone(),
        tentativa,
        geracao,
    };
    if let Err(error) = executar(state, &trabalho, &contexto).await {
        // Uma tentativa antiga não pode gravar no banco restaurado nem em uma retomada.
        let _leitura = state.backup_runtime.leitura().await;
        if state.backup_runtime.geracao() == contexto.geracao {
            sqlx::query("UPDATE osint_trabalho SET estado='erro',erro=?,tentativa=NULL,atualizado_em=CURRENT_TIMESTAMP WHERE id=? AND tentativa=? AND estado='executando'")
                .bind(error.to_string()).bind(&contexto.id).bind(&contexto.tentativa).execute(&state.pool).await?;
        }
    }
    Ok(())
}
async fn executar(
    state: &AppState,
    trabalho: &Trabalho,
    contexto: &Contexto,
) -> Result<(), AppError> {
    let parametros: Vec<ParametroBusca> =
        serde_json::from_str(&trabalho.parametros).map_err(|e| AppError::interno(e.to_string()))?;
    let mut total: VarreduraResponse =
        serde_json::from_str(&trabalho.resultado).map_err(|e| AppError::interno(e.to_string()))?;
    let mut providers =
        super::PublicSearchProviders::new(&state.config).map_err(AppError::interno)?;
    let stored: serde_json::Value =
        serde_json::from_str(&trabalho.resultado).map_err(|e| AppError::interno(e.to_string()))?;
    let mut fontes: std::collections::BTreeSet<String> = stored["fontes_indisponiveis_nomes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    for parametro in parametros.into_iter().skip(trabalho.processados as usize) {
        {
            let _leitura = contexto.validar(state).await?;
        }
        let (resultado, indisponiveis) = super::executar_parametros(
            state.clone(),
            trabalho.pessoa_id,
            vec![parametro],
            contexto,
            &mut providers,
        )
        .await?;
        total.parametros_processados += resultado.parametros_processados;
        total.parametros_inconclusivos += resultado.parametros_inconclusivos;
        total.resultados_encontrados += resultado.resultados_encontrados;
        total.novos_achados += resultado.novos_achados;
        total.pdfs_arquivados += resultado.pdfs_arquivados;
        fontes.extend(indisponiveis);
        total.fontes_indisponiveis = fontes.len();
        total.avisos.extend(resultado.avisos);
        total.situacao = super::classificar_varredura(
            total.parametros_processados,
            total.parametros_inconclusivos,
            total.fontes_indisponiveis > 0 || !total.avisos.is_empty(),
        )
        .into();
        let _leitura = contexto.validar(state).await?;
        let mut progresso =
            serde_json::to_value(&total).map_err(|e| AppError::interno(e.to_string()))?;
        progresso["fontes_indisponiveis_nomes"] = serde_json::json!(fontes);
        sqlx::query("UPDATE osint_trabalho SET processados=?,resultado=?,atualizado_em=? WHERE id=? AND tentativa=? AND estado='executando'")
            .bind(total.parametros_processados as i64).bind(progresso.to_string()).bind(Utc::now().to_rfc3339()).bind(&contexto.id).bind(&contexto.tentativa).execute(&state.pool).await?;
    }
    let _leitura = contexto.validar(state).await?;
    sqlx::query("UPDATE osint_trabalho SET estado='concluido',tentativa=NULL,atualizado_em=CURRENT_TIMESTAMP WHERE id=? AND tentativa=? AND estado='executando'").bind(&contexto.id).bind(&contexto.tentativa).execute(&state.pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[tokio::test]
    async fn worker_retoma_checkpoint_e_cancela_consulta_sem_persistir_resposta_tardia() {
        let consultas = Arc::new(AtomicUsize::new(0));
        let chamada = Arc::new(tokio::sync::Notify::new());
        let contador = consultas.clone();
        let aviso = chamada.clone();
        let mock=axum::Router::new().route("/search",axum::routing::get(move |axum::extract::Query(query):axum::extract::Query<std::collections::HashMap<String,String>>| {
            let contador=contador.clone();let aviso=aviso.clone();
            async move {
                contador.fetch_add(1,Ordering::Relaxed);
                let q=query.get("q").cloned().unwrap();
                if q=="lenta" {aviso.notify_one();tokio::time::sleep(std::time::Duration::from_secs(3)).await;}
                Json(serde_json::json!({"results":[{"title":q,"url":format!("https://example.org/{q}"),"content":"Achado de teste","engine":"mock"}]}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
        let mut config = crate::config::Config::from_env().unwrap();
        config.database_url = "sqlite::memory:".into();
        config.admin_login = Some("worker".into());
        config.admin_password = Some("senha-worker-segura".into());
        config.searxng_url = Some(url);
        let pool = crate::db::conectar(&config).await.unwrap();
        let state = AppState {
            pool: pool.clone(),
            config,
            auth_runtime: crate::handlers::auth::AuthRuntime::default(),
            backup_runtime: crate::handlers::backup::BackupRuntime::default(),
        };
        let uid: i64 = sqlx::query_scalar("SELECT id FROM usuario WHERE login='worker'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let pid: i64 =
            sqlx::query_scalar("INSERT INTO pessoa(nome) VALUES('Pesquisa de teste') RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();
        let params = vec![
            ParametroBusca {
                id: 1,
                pessoa_id: pid,
                tipo: "TERMO".into(),
                valor: "primeira".into(),
                provider: "SEARXNG".into(),
                ativo: true,
            },
            ParametroBusca {
                id: 2,
                pessoa_id: pid,
                tipo: "TERMO".into(),
                valor: "segunda".into(),
                provider: "SEARXNG".into(),
                ativo: true,
            },
        ];
        let mut parcial = vazio();
        parcial.parametros_processados = 1;
        parcial.novos_achados = 1;
        sqlx::query("INSERT INTO historico_busca_publica(pessoa_id,fonte,parametro_utilizado,titulo_resultado,url_origem) VALUES(?,'mock','TERMO: primeira','primeira','https://example.org/primeira')").bind(pid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO osint_trabalho(id,usuario_id,pessoa_id,estado,parametros,total,processados,resultado) VALUES('checkpoint',?,?,'fila',?,2,1,?)").bind(uid).bind(pid).bind(serde_json::to_string(&params).unwrap()).bind(serde_json::to_string(&parcial).unwrap()).execute(&pool).await.unwrap();
        proximo(&state).await.unwrap();
        assert_eq!(consultas.load(Ordering::Relaxed), 1);
        assert_eq!(
            sqlx::query_as::<_, (String, i64)>(
                "SELECT estado,processados FROM osint_trabalho WHERE id='checkpoint'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            ("concluido".into(), 2)
        );
        let lenta = vec![ParametroBusca {
            valor: "lenta".into(),
            ..params[0].clone()
        }];
        sqlx::query("INSERT INTO osint_trabalho(id,usuario_id,pessoa_id,estado,parametros,total,resultado) VALUES('cancelar',?,?,'fila',?,1,?)").bind(uid).bind(pid).bind(serde_json::to_string(&lenta).unwrap()).bind(serde_json::to_string(&vazio()).unwrap()).execute(&pool).await.unwrap();
        let background = state.clone();
        let task = tokio::spawn(async move { proximo(&background).await.unwrap() });
        tokio::time::timeout(std::time::Duration::from_secs(5), chamada.notified())
            .await
            .unwrap();
        sqlx::query(
            "UPDATE osint_trabalho SET estado='cancelado',tentativa=NULL WHERE id='cancelar'",
        )
        .execute(&pool)
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM historico_busca_publica")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        let antiga = Contexto {
            id: "checkpoint".into(),
            tentativa: "antiga".into(),
            geracao: 1,
        };
        assert!(antiga.validar(&state).await.is_err());
        server.abort();
    }
}
