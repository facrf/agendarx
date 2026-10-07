use crate::{AppState, error::AppError, middleware::auth::SessaoAutenticada};
use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;
use sqlx::{Sqlite, Transaction};
type LinhaVinculo = (i64, i64, i64, String, Option<String>, Option<String>);

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/previa", get(previa))
        .route("/confirmar", axum::routing::post(confirmar))
}
#[derive(Deserialize)]
struct Par {
    origem: i64,
    destino: i64,
}
#[derive(Deserialize)]
struct Input {
    origem: i64,
    destino: i64,
    versao_origem: i64,
    versao_destino: i64,
    #[serde(default)]
    usar_origem: Vec<String>,
}
async fn verificar_par(
    tx: &mut Transaction<'_, Sqlite>,
    origem: i64,
    destino: i64,
) -> Result<(), AppError> {
    if origem == destino {
        return Err(AppError::BadRequest(
            "Escolha duas pessoas diferentes".into(),
        ));
    }
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pessoa WHERE id IN (?,?) AND excluida_em IS NULL")
            .bind(origem)
            .bind(destino)
            .fetch_one(&mut **tx)
            .await?;
    if count != 2 {
        return Err(AppError::nao_encontrado("duas pessoas ativas"));
    }
    Ok(())
}
async fn previa(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Query(par): Query<Par>,
) -> Result<Json<serde_json::Value>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    let mut tx = state.pool.begin().await?;
    verificar_par(&mut tx, par.origem, par.destino).await?;
    let origem = super::revisoes::capturar(&mut tx, "pessoa", par.origem).await?;
    let destino = super::revisoes::capturar(&mut tx, "pessoa", par.destino).await?;
    let a = super::revisoes::versao(&mut tx, "pessoa", par.origem, sessao.usuario.id).await?;
    let b = super::revisoes::versao(&mut tx, "pessoa", par.destino, sessao.usuario.id).await?;
    let anexos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM anexo_dossie WHERE pessoa_id=?")
        .bind(par.origem)
        .fetch_one(&mut *tx)
        .await?;
    let vinculos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pessoa_vinculo WHERE pessoa_origem_id=? OR pessoa_destino_id=?",
    )
    .bind(par.origem)
    .bind(par.origem)
    .fetch_one(&mut *tx)
    .await?;
    Ok(Json(
        serde_json::json!({"origem":serde_json::from_str::<serde_json::Value>(&origem).map_err(|e|AppError::interno(e.to_string()))?,"destino":serde_json::from_str::<serde_json::Value>(&destino).map_err(|e|AppError::interno(e.to_string()))?,"versao_origem":a,"versao_destino":b,"anexos":anexos,"vinculos":vinculos}),
    ))
}
async fn confirmar(
    State(state): State<AppState>,
    Extension(sessao): Extension<SessaoAutenticada>,
    Json(input): Json<Input>,
) -> Result<Json<serde_json::Value>, AppError> {
    if sessao.usuario.perfil != "admin" {
        return Err(AppError::Forbidden);
    }
    let allowed = [
        "nome",
        "descricao",
        "categoria_id",
        "pessoa_juridica",
        "foto_principal",
        "risco",
    ];
    if input
        .usar_origem
        .iter()
        .any(|f| !allowed.contains(&f.as_str()))
    {
        return Err(AppError::BadRequest("campo de mesclagem inválido".into()));
    }
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    verificar_par(&mut tx, input.origem, input.destino).await?;
    super::revisoes::verificar(
        &mut tx,
        "pessoa",
        input.origem,
        sessao.usuario.id,
        Some(input.versao_origem),
    )
    .await?;
    super::revisoes::verificar(
        &mut tx,
        "pessoa",
        input.destino,
        sessao.usuario.id,
        Some(input.versao_destino),
    )
    .await?;
    let a = super::revisoes::capturar(&mut tx, "pessoa", input.origem).await?;
    let b = super::revisoes::capturar(&mut tx, "pessoa", input.destino).await?;
    sqlx::query("INSERT INTO mesclagem_registro(origem_id,destino_id,autor_login,origem_json,destino_json) VALUES(?,?,?,?,?)").bind(input.origem).bind(input.destino).bind(&sessao.usuario.login).bind(&a).bind(&b).execute(&mut *tx).await?;
    let mut report = format!(
        "# Mesclagem de contatos\n\nAutor: {}\nData: {}\nOrigem: #{}\nDestino: #{}\n",
        sessao.usuario.login,
        chrono::Utc::now().to_rfc3339(),
        input.origem,
        input.destino
    );
    for (label, snapshot) in [("Origem", &a), ("Destino", &b)] {
        let data: serde_json::Value =
            serde_json::from_str(snapshot).map_err(|e| AppError::interno(e.to_string()))?;
        report.push_str(&format!("\n## {label} antes da mesclagem\n\nNome: {}\nCategoria: {}\nDescrição:\n{}\n\nAvaliação: {} · {}\nJustificativa: {}\n",data["nome"].as_str().unwrap_or(""),data["categoria_nome"].as_str().unwrap_or("Sem categoria"),data["descricao"].as_str().unwrap_or(""),data["classificacao_risco"].as_str().unwrap_or(""),data["toxicidade"],data["risco_justificativa"].as_str().unwrap_or("")));
        for contact in data["contatos"].as_array().into_iter().flatten() {
            report.push_str(&format!(
                "- Contato: {}\n",
                contact["valor"].as_str().unwrap_or("")
            ));
        }
    }
    sqlx::query("INSERT INTO anexo_dossie(pessoa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) VALUES(?,?,'text/markdown',?,?, 'Dados anteriores à mesclagem')").bind(input.destino).bind(format!("mesclagem-{}-{}.md",input.origem,input.destino)).bind(report.as_bytes()).bind(report.len() as i64).execute(&mut *tx).await?;
    // Preserve both profile photos as dossier attachments before selecting the principal photo.
    for id in [input.origem, input.destino] {
        let photo: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT foto_principal FROM pessoa WHERE id=?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if let Some(photo) = photo {
            let mime = infer::get(&photo)
                .map(|t| t.mime_type())
                .unwrap_or("application/octet-stream");
            let ext = infer::get(&photo).map(|t| t.extension()).unwrap_or("bin");
            sqlx::query("INSERT INTO anexo_dossie(pessoa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) VALUES(?,?,?,?,?,'Foto preservada antes da mesclagem')").bind(input.destino).bind(format!("foto-perfil-mesclagem-{id}.{ext}")).bind(mime).bind(&photo).bind(photo.len() as i64).execute(&mut *tx).await?;
        }
    }
    let risco_anterior = super::hp_psicossocial::carregar_registro(&mut tx, input.destino).await?;
    for field in &input.usar_origem {
        let fields: Vec<&str> = if field == "risco" {
            vec![
                "classificacao_risco",
                "toxicidade",
                "risco_justificativa",
                "risco_revisado_em",
            ]
        } else {
            vec![field.as_str()]
        };
        for f in fields {
            sqlx::query(&format!(
                "UPDATE pessoa SET {f}=(SELECT {f} FROM pessoa WHERE id=?) WHERE id=?"
            ))
            .bind(input.origem)
            .bind(input.destino)
            .execute(&mut *tx)
            .await?;
        }
    }
    if input.usar_origem.iter().any(|f| f == "risco") {
        let novo = super::hp_psicossocial::carregar_registro(&mut tx, input.destino).await?;
        sqlx::query("INSERT INTO pessoa_risco_historico(pessoa_id,autor_id,autor_login,anterior_json,novo_json) VALUES(?,?,?,?,?)").bind(input.destino).bind(sessao.usuario.id).bind(&sessao.usuario.login).bind(serde_json::to_string(&risco_anterior).map_err(|e|AppError::interno(e.to_string()))?).bind(serde_json::to_string(&novo).map_err(|e|AppError::interno(e.to_string()))?).execute(&mut *tx).await?;
    }
    // Exact duplicate contact values are retained once; every distinct value remains available.
    sqlx::query("DELETE FROM contato WHERE pessoa_id=? AND EXISTS(SELECT 1 FROM contato c WHERE c.pessoa_id=? AND c.tipo_contato_id=contato.tipo_contato_id AND lower(trim(c.valor))=lower(trim(contato.valor)))").bind(input.origem).bind(input.destino).execute(&mut *tx).await?;
    for table in ["contato", "anexo_dossie"] {
        sqlx::query(&format!("UPDATE {table} SET pessoa_id=? WHERE pessoa_id=?"))
            .bind(input.destino)
            .bind(input.origem)
            .execute(&mut *tx)
            .await?;
    }
    for table in [
        "pessoa_etiqueta",
        "pessoa_favorita",
        "tarefa_calendario_pessoa",
    ] {
        let key = match table {
            "pessoa_etiqueta" => "etiqueta_id",
            "pessoa_favorita" => "usuario_id",
            _ => "tarefa_id",
        };
        sqlx::query(&format!("INSERT OR IGNORE INTO {table}(pessoa_id,{key}) SELECT ?,{key} FROM {table} WHERE pessoa_id=?")).bind(input.destino).bind(input.origem).execute(&mut *tx).await?;
        sqlx::query(&format!("DELETE FROM {table} WHERE pessoa_id=?"))
            .bind(input.origem)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT OR IGNORE INTO parametro_busca(pessoa_id,tipo,valor,ativo,provider) SELECT ?,tipo,valor,ativo,provider FROM parametro_busca WHERE pessoa_id=?").bind(input.destino).bind(input.origem).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM parametro_busca WHERE pessoa_id=?")
        .bind(input.origem)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE historico_busca_publica SET pessoa_id=? WHERE pessoa_id=? AND NOT EXISTS(SELECT 1 FROM historico_busca_publica d WHERE d.pessoa_id=? AND d.url_origem=historico_busca_publica.url_origem)").bind(input.destino).bind(input.origem).bind(input.destino).execute(&mut *tx).await?;
    let edges:Vec<LinhaVinculo>=sqlx::query_as("SELECT id,pessoa_origem_id,pessoa_destino_id,tipo_vinculo,descricao,excluido_em FROM pessoa_vinculo WHERE pessoa_origem_id=? OR pessoa_destino_id=?").bind(input.origem).bind(input.origem).fetch_all(&mut *tx).await?;
    for (id, from, to, kind, description, deleted) in edges {
        let from = if from == input.origem {
            input.destino
        } else {
            from
        };
        let to = if to == input.origem {
            input.destino
        } else {
            to
        };
        let duplicate:Option<i64>=sqlx::query_scalar("SELECT id FROM pessoa_vinculo WHERE pessoa_origem_id=? AND pessoa_destino_id=? AND tipo_vinculo=? AND id<>?").bind(from).bind(to).bind(&kind).bind(id).fetch_optional(&mut *tx).await?;
        if from == to || duplicate.is_some() {
            if let Some(duplicate) = duplicate
                && deleted.is_none()
            {
                sqlx::query("UPDATE pessoa_vinculo SET excluido_em=NULL WHERE id=?")
                    .bind(duplicate)
                    .execute(&mut *tx)
                    .await?;
            }
            // Archive notes and files of collapsed/self links in the surviving person's dossier.
            sqlx::query("INSERT INTO anexo_dossie(pessoa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) SELECT ?,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas FROM anexo_vinculo WHERE vinculo_id=?").bind(input.destino).bind(id).execute(&mut *tx).await?;
            let note = format!(
                "Vínculo #{id}: {kind}\n{}\nEstado anterior: {}",
                description.unwrap_or_default(),
                if deleted.is_some() {
                    "na lixeira"
                } else {
                    "ativo"
                }
            );
            sqlx::query("INSERT INTO anexo_dossie(pessoa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) VALUES(?,?,'text/markdown',?,?, 'Vínculo arquivado durante a mesclagem')").bind(input.destino).bind(format!("vinculo-{id}-mesclagem.md")).bind(note.as_bytes()).bind(note.len() as i64).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM pessoa_vinculo WHERE id=?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                "UPDATE pessoa_vinculo SET pessoa_origem_id=?,pessoa_destino_id=? WHERE id=?",
            )
            .bind(from)
            .bind(to)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }
    }
    sqlx::query("UPDATE osint_trabalho SET estado='interrompido',tentativa=NULL,erro='Pessoa mesclada',atualizado_em=CURRENT_TIMESTAMP WHERE pessoa_id IN (?,?) AND estado IN ('fila','executando')").bind(input.origem).bind(input.destino).execute(&mut *tx).await?;
    sqlx::query("UPDATE pessoa SET excluida_em=CURRENT_TIMESTAMP,mesclada_em=CURRENT_TIMESTAMP,mesclada_destino_id=? WHERE id=?")
        .bind(input.destino)
        .bind(input.origem)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE pessoa SET mesclagem_revisao_limite=(SELECT COALESCE(MAX(id),0) FROM revisao_edicao) WHERE id=?").bind(input.destino).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"pessoa_id":input.destino})))
}
