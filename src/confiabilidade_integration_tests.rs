use crate::{integration_tests::TestApi, workflow_integration_tests::fixture};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
async fn get(api: &TestApi, path: &str, token: &str) -> Value {
    api.json(Method::GET, path, token, Value::Null, StatusCode::OK)
        .await
}
async fn edit(
    api: &TestApi,
    path: &str,
    token: &str,
    version: i64,
    body: Value,
    status: StatusCode,
) -> Value {
    let response = api
        .client
        .put(format!("{}{path}", api.base))
        .bearer_auth(token)
        .header("if-match", version)
        .json(&body)
        .send()
        .await
        .unwrap();
    let actual = response.status();
    let text = response.text().await.unwrap();
    assert_eq!(actual, status, "{text}");
    serde_json::from_str(&text).unwrap()
}
#[tokio::test]
async fn edicoes_concorrentes_nao_sobrescrevem_e_desfazer_preserva_o_estado_atual() {
    let (api, pool, admin, _, _, _) = fixture().await;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO pessoa(nome,descricao) VALUES('Original','Antes') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO contato(pessoa_id,tipo_contato_id,valor) VALUES(?,1,'123')")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let path = format!("/api/pessoas/{id}");
    let version = get(&api, &path, &admin).await["versao"].as_i64().unwrap();
    let request = |name: &str| {
        api.client
            .put(format!("{}{path}", api.base))
            .bearer_auth(&admin)
            .header("if-match", version)
            .json(&json!({"nome":name,"descricao":"Depois","pessoa_juridica":false}))
            .send()
    };
    let (a, b) = tokio::join!(request("Primeira"), request("Segunda"));
    let mut statuses = vec![a.unwrap().status().as_u16(), b.unwrap().status().as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let history = get(&api, &format!("/api/revisoes/pessoa/{id}"), &admin).await;
    assert_eq!(history["itens"].as_array().unwrap().len(), 1);
    let current = get(&api, &path, &admin).await;
    let revision = history["itens"][0]["id"].as_i64().unwrap();
    let undo = format!("/api/revisoes/pessoa/{id}/{revision}/restaurar");
    api.json(
        Method::POST,
        &undo,
        &admin,
        json!({"versao":version}),
        StatusCode::CONFLICT,
    )
    .await;
    api.json(
        Method::POST,
        &undo,
        &admin,
        json!({"versao":current["versao"]}),
        StatusCode::OK,
    )
    .await;
    let restored = get(&api, &path, &admin).await;
    assert_eq!(restored["nome"], "Original");
    assert_eq!(restored["contatos"][0]["valor"], "123");
    assert_eq!(
        get(&api, &format!("/api/revisoes/pessoa/{id}"), &admin).await["itens"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let contact = restored["contatos"][0]["id"].as_i64().unwrap();
    edit(
        &api,
        &format!("/api/pessoas/contatos/{contact}"),
        &admin,
        restored["versao"].as_i64().unwrap(),
        json!({"tipo_contato_id":1,"valor":"456"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        get(&api, &format!("/api/revisoes/pessoa/{id}"), &admin).await["itens"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
#[tokio::test]
async fn historico_privado_e_versoes_de_tarefas_e_vinculos() {
    let (api, pool, admin, user, admin_id, _) = fixture().await;
    let task:i64=sqlx::query_scalar("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em) VALUES(?,'Antes','2026-10-07T12:00:00Z') RETURNING id").bind(admin_id).fetch_one(&pool).await.unwrap();
    api.json(
        Method::PATCH,
        &format!("/api/calendario/tarefas/{task}/status"),
        &admin,
        json!({"status":"CONCLUIDA"}),
        StatusCode::OK,
    )
    .await;
    let history = format!("/api/revisoes/tarefa/{task}");
    api.json(
        Method::GET,
        &history,
        &user,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    let h = get(&api, &history, &admin).await;
    let undo = format!("{history}/{}/restaurar", h["itens"][0]["id"]);
    api.json(
        Method::POST,
        &undo,
        &user,
        json!({"versao":h["versao"]}),
        StatusCode::NOT_FOUND,
    )
    .await;
    api.json(
        Method::POST,
        &undo,
        &admin,
        json!({"versao":h["versao"]}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        get(&api, &format!("/api/calendario/tarefas/{task}"), &admin).await["status"],
        "PENDENTE"
    );
    sqlx::query("INSERT INTO pessoa(nome) VALUES('A'),('B')")
        .execute(&pool)
        .await
        .unwrap();
    let edge = get(&api, "/api/vinculos", &admin).await;
    assert_eq!(edge.as_array().unwrap().len(), 0);
    let edge=api.json(Method::POST,"/api/vinculos",&admin,json!({"pessoa_origem_id":1,"pessoa_destino_id":2,"tipo_vinculo":"Família","descricao":"Antes"}),StatusCode::CREATED).await;
    let path = format!("/api/vinculos/{}", edge["id"]);
    let body = json!({"pessoa_origem_id":1,"pessoa_destino_id":2,"tipo_vinculo":"Família","descricao":"Depois"});
    let updated = edit(
        &api,
        &path,
        &admin,
        edge["versao"].as_i64().unwrap(),
        body.clone(),
        StatusCode::OK,
    )
    .await;
    assert!(updated["versao"].as_i64() > edge["versao"].as_i64());
    edit(
        &api,
        &path,
        &admin,
        edge["versao"].as_i64().unwrap(),
        body,
        StatusCode::CONFLICT,
    )
    .await;
}
#[tokio::test]
async fn mesclagem_revalida_previa_transfere_dependencias_e_arquiva_vinculos() {
    let (api, pool, admin, user, admin_id, _) = fixture().await;
    sqlx::query("INSERT INTO pessoa(nome,descricao) VALUES('Origem','Descrição da origem'),('Destino','Descrição do destino'),('Outra','')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO contato(pessoa_id,tipo_contato_id,valor) VALUES(1,1,'123'),(2,1,'123'),(1,1,'456')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO pessoa_vinculo(pessoa_origem_id,pessoa_destino_id,tipo_vinculo,descricao) VALUES(1,2,'Família','Vínculo interno'),(1,3,'Família','Origem'),(2,3,'Família','Destino')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO anexo_vinculo(vinculo_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) VALUES(1,'arquivo.txt','text/plain',x'78',1,'Notas preservadas')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO anexo_dossie(pessoa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes) VALUES(1,'origem.txt','text/plain',x'78',1)").execute(&pool).await.unwrap();
    let task:i64=sqlx::query_scalar("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em) VALUES(?,'Privada','2026-10-07T12:00:00Z') RETURNING id").bind(admin_id).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO tarefa_calendario_pessoa(tarefa_id,pessoa_id) VALUES(?,1),(?,2)")
        .bind(task)
        .bind(task)
        .execute(&pool)
        .await
        .unwrap();
    let preview = get(&api, "/api/mesclagem/previa?origem=1&destino=2", &admin).await;
    let input = json!({"origem":1,"destino":2,"versao_origem":preview["versao_origem"],"versao_destino":preview["versao_destino"],"usar_origem":["nome"]});
    api.json(
        Method::POST,
        "/api/mesclagem/confirmar",
        &user,
        input.clone(),
        StatusCode::FORBIDDEN,
    )
    .await;
    sqlx::query("UPDATE pessoa SET descricao='Atualizada' WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    api.json(
        Method::POST,
        "/api/mesclagem/confirmar",
        &admin,
        input,
        StatusCode::CONFLICT,
    )
    .await;
    let preview = get(&api, "/api/mesclagem/previa?origem=1&destino=2", &admin).await;
    api.json(Method::POST,"/api/mesclagem/confirmar",&admin,json!({"origem":1,"destino":2,"versao_origem":preview["versao_origem"],"versao_destino":preview["versao_destino"],"usar_origem":["nome"]}),StatusCode::OK).await;
    let merged = get(&api, "/api/pessoas/2", &admin).await;
    assert_eq!(merged["nome"], "Origem");
    assert_eq!(merged["descricao"], "Descrição do destino");
    assert_eq!(merged["contatos"].as_array().unwrap().len(), 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pessoa_vinculo")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM anexo_dossie WHERE pessoa_id=2")
            .fetch_one(&pool)
            .await
            .unwrap(),
        5
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tarefa_calendario_pessoa WHERE tarefa_id=? AND pessoa_id=2"
        )
        .bind(task)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    api.json(
        Method::POST,
        "/api/produtividade/lixeira/1/restaurar",
        &admin,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT mesclada_destino_id FROM pessoa WHERE id=1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mesclagem_registro")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn preferencias_adiamento_e_ack_tardio_respeitam_conta_e_versao() {
    let (api, pool, admin, user, admin_id, _) = fixture().await;
    api.json(
        Method::PUT,
        "/api/preferencias/lembretes",
        &admin,
        json!({"silencioso_inicio":"22:00","silencioso_fim":"07:00"}),
        StatusCode::OK,
    )
    .await;
    assert!(get(&api, "/api/preferencias/lembretes", &user).await["silencioso_inicio"].is_null());
    api.json(
        Method::PUT,
        "/api/preferencias/lembretes",
        &admin,
        json!({"silencioso_inicio":"22:00","silencioso_fim":"22:00"}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let task:i64=sqlx::query_scalar("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em,lembrete_minutos) VALUES(?,'Lembrete',?,0) RETURNING id").bind(admin_id).bind(now).fetch_one(&pool).await.unwrap();
    let before = get(&api, &format!("/api/calendario/tarefas/{task}"), &admin).await;
    sqlx::query(
        "UPDATE tarefa_calendario SET titulo='Título alterado no mesmo segundo' WHERE id=?",
    )
    .bind(task)
    .execute(&pool)
    .await
    .unwrap();
    api.json(Method::PATCH,&format!("/api/calendario/lembretes/{task}/dispensar"),&admin,json!({"versao":before["versao"],"inicio_em":before["inicio_em"],"lembrete_minutos":0,"data_atualizacao":before["data_atualizacao"]}),StatusCode::CONFLICT).await;
    let before = get(&api, &format!("/api/calendario/tarefas/{task}"), &admin).await;
    let snooze = format!("/api/preferencias/lembretes/{task}/adiar");
    api.json(
        Method::POST,
        &snooze,
        &user,
        json!({"minutos":15,"versao":before["versao"]}),
        StatusCode::NOT_FOUND,
    )
    .await;
    api.json(
        Method::POST,
        &snooze,
        &admin,
        json!({"minutos":15,"versao":before["versao"]}),
        StatusCode::OK,
    )
    .await;
    api.json(Method::PATCH,&format!("/api/calendario/lembretes/{task}/dispensar"),&admin,json!({"inicio_em":before["inicio_em"],"lembrete_minutos":0,"data_atualizacao":before["data_atualizacao"]}),StatusCode::CONFLICT).await;
    assert_eq!(
        get(&api, "/api/calendario/lembretes", &admin)
            .await
            .as_array()
            .unwrap()
            .len(),
        0
    );
    sqlx::query(
        "UPDATE tarefa_calendario SET lembrete_adiado_ate='2000-01-01T00:00:00Z' WHERE id=?",
    )
    .bind(task)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        get(&api, "/api/calendario/lembretes", &admin)
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    sqlx::query("UPDATE tarefa_calendario SET status='CONCLUIDA' WHERE id=?")
        .bind(task)
        .execute(&pool)
        .await
        .unwrap();
    let completed = get(&api, &format!("/api/calendario/tarefas/{task}"), &admin).await;
    api.json(
        Method::POST,
        &snooze,
        &admin,
        json!({"minutos":15,"versao":completed["versao"]}),
        StatusCode::CONFLICT,
    )
    .await;
    let health = get(&api, "/api/saude", &admin).await;
    assert!(health["backup"]["ultimo_validado"].is_null());
    api.json(
        Method::GET,
        "/api/saude",
        &user,
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
    api.json(
        Method::POST,
        "/api/saude/backup/verificar",
        &user,
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
}

#[tokio::test]
async fn painel_detecta_backup_corrompido_e_remove_da_lista_de_validos() {
    use crate::{
        AppState,
        config::Config,
        db,
        handlers::{
            auth::AuthRuntime,
            backup::{self, BackupRuntime},
        },
    };
    use std::io::Write;
    let dir = std::path::PathBuf::from(format!(".cache/health-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut config = Config::from_env().unwrap();
    config.database_url = format!("sqlite://{}?mode=rwc", dir.join("app.db").display());
    config.admin_login = Some("admin-saude".into());
    config.admin_password = Some("senha-saude-segura".into());
    let pool = db::conectar(&config).await.unwrap();
    let state = AppState {
        pool: pool.clone(),
        config,
        auth_runtime: AuthRuntime::default(),
        backup_runtime: BackupRuntime::default(),
    };
    let info = backup::criar_snapshot(&state, false).await.unwrap();
    assert_eq!(
        backup::verificar_ultimo_backup(&state).await.unwrap()["integridade_ok"],
        true
    );
    let path = dir.join("backups").join(&info.nome_arquivo);
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"checksum-alterado")
        .unwrap();
    assert_eq!(
        backup::verificar_ultimo_backup(&state).await.unwrap()["integridade_ok"],
        false
    );
    assert!(backup::resumo_saude(&state).await.unwrap()["ultimo_validado"].is_null());
    pool.close().await;
    std::fs::remove_dir_all(dir).unwrap();
}
