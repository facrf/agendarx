use crate::{integration_tests::TestApi, workflow_integration_tests::fixture};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

async fn arquivo(
    api: &TestApi,
    path: &str,
    token: &str,
    dados: &[u8],
    status: StatusCode,
) -> Value {
    let mut body=b"--melhoria\r\nContent-Disposition: form-data; name=\"arquivo\"; filename=\"agenda.csv\"\r\nContent-Type: text/csv\r\n\r\n".to_vec();
    body.extend_from_slice(dados);
    body.extend_from_slice(b"\r\n--melhoria--\r\n");
    let res = api
        .client
        .post(format!("{}{path}", api.base))
        .bearer_auth(token)
        .header("content-type", "multipart/form-data; boundary=melhoria")
        .body(body)
        .send()
        .await
        .unwrap();
    let actual = res.status();
    let text = res.text().await.unwrap();
    assert_eq!(actual, status, "{path}: {text}");
    serde_json::from_str(&text).unwrap_or(Value::Null)
}
#[tokio::test]
async fn exportacao_e_previa_importacao_preservam_dados_e_revalidam_coincidencias() {
    let (api, pool, admin, _, _, _) = fixture().await;
    let id:i64=sqlx::query_scalar("INSERT INTO pessoa(nome,descricao) VALUES('Maria original','Descrição preservada') RETURNING id").fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO contato(pessoa_id,tipo_contato_id,valor) VALUES(?,(SELECT id FROM tipo_meio_contato WHERE nome_tipo LIKE '%mail%' LIMIT 1),'MARIA@example.com')").bind(id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO pessoa(nome,excluida_em) VALUES('Excluída',CURRENT_TIMESTAMP)")
        .execute(&pool)
        .await
        .unwrap();
    for formato in ["csv", "vcf"] {
        let res = api
            .client
            .get(format!(
                "{}/api/configuracoes/contatos/exportar/{formato}",
                api.base
            ))
            .bearer_auth(&admin)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(res.headers().contains_key("content-disposition"));
        let data = res.text().await.unwrap();
        assert!(data.contains("Maria original"));
        assert!(!data.contains("Excluída"));
    }
    let csv=b"Name,E-mail 1 - Value,Phone 1 - Value\r\nMaria importada,maria@example.com,+55 (11) 99999-1234\r\nNovo,novo@example.com,\r\nRepetido,novo@example.com,\r\n";
    let preview = arquivo(
        &api,
        "/api/configuracoes/contatos/importar/previa",
        &admin,
        csv,
        StatusCode::OK,
    )
    .await;
    assert_eq!(preview["registros"][0]["coincidencias"][0]["pessoa_id"], id);
    assert_eq!(preview["registros"][1]["repetidos_no_arquivo"], json!([2]));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pessoa")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    let path = format!(
        "/api/configuracoes/contatos/importacoes/{}/confirmar",
        preview["token"].as_str().unwrap()
    );
    let result=api.json(Method::POST,&path,&admin,json!({"decisoes":[{"indice":0,"acao":"atualizar","pessoa_id":id},{"indice":1,"acao":"criar"},{"indice":2,"acao":"ignorar"}]}),StatusCode::OK).await;
    assert_eq!(result["pessoas_atualizadas"], 1);
    assert_eq!(result["pessoas_importadas"], 1);
    assert_eq!(result["contatos_importados"], 2);
    assert_eq!(
        sqlx::query_as::<_, (String, String)>("SELECT nome,descricao FROM pessoa WHERE id=?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        ("Maria original".into(), "Descrição preservada".into())
    );
    api.json(
        Method::POST,
        &path,
        &admin,
        json!({"decisoes":[]}),
        StatusCode::NOT_FOUND,
    )
    .await;
    let preview = arquivo(
        &api,
        "/api/configuracoes/contatos/importar/previa",
        &admin,
        csv,
        StatusCode::OK,
    )
    .await;
    sqlx::query("DELETE FROM contato WHERE pessoa_id=?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    api.json(Method::POST,&format!("/api/configuracoes/contatos/importacoes/{}/confirmar",preview["token"].as_str().unwrap()),&admin,json!({"decisoes":[{"indice":0,"acao":"criar"},{"indice":1,"acao":"ignorar"},{"indice":2,"acao":"ignorar"}]}),StatusCode::CONFLICT).await;
}
#[tokio::test]
async fn trabalhos_osint_sao_persistentes_isolados_cancelaveis_e_retomaveis() {
    let (api, pool, admin, user, _, _) = fixture().await;
    let id: i64 = sqlx::query_scalar("INSERT INTO pessoa(nome) VALUES('Pesquisa') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO parametro_busca(pessoa_id,tipo,valor,provider,ativo) VALUES(?,'NOME','Pesquisa','SEARXNG',1)").bind(id).execute(&pool).await.unwrap();
    let job = api
        .json(
            Method::POST,
            &format!("/api/osint/varrer/{id}"),
            &admin,
            Value::Null,
            StatusCode::ACCEPTED,
        )
        .await;
    assert_eq!(job["estado"], "fila");
    assert_eq!(job["total"], 1);
    let list = api
        .json(
            Method::GET,
            &format!("/api/osint/trabalhos/pessoa/{id}"),
            &user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert!(list.as_array().unwrap().is_empty());
    let jid = job["id"].as_str().unwrap();
    api.json(
        Method::POST,
        &format!("/api/osint/trabalhos/{jid}/cancelar"),
        &user,
        Value::Null,
        StatusCode::CONFLICT,
    )
    .await;
    api.json(
        Method::POST,
        &format!("/api/osint/trabalhos/{jid}/cancelar"),
        &admin,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    api.json(
        Method::POST,
        &format!("/api/osint/trabalhos/{jid}/retomar"),
        &admin,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    crate::handlers::osint::interromper_trabalhos(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT estado FROM osint_trabalho WHERE id=?")
            .bind(jid)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "interrompido"
    );
    api.json(
        Method::POST,
        &format!("/api/osint/trabalhos/{jid}/retomar"),
        &admin,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
}
#[tokio::test]
async fn limites_login_retornam_429_com_retry_after_e_413_para_corpo_grande() {
    let (api, _, _, _, _, _) = fixture().await;
    let res = api
        .client
        .post(format!("{}/api/auth/login", api.base))
        .json(&json!({"login":"inexistente","senha":"x".repeat(17000)}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    for _ in 0..10 {
        api.json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"inexistente","senha":"errada"}),
            StatusCode::UNAUTHORIZED,
        )
        .await;
    }
    let res = api
        .client
        .post(format!("{}/api/auth/login", api.base))
        .json(&json!({"login":"inexistente","senha":"errada"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        res.headers()["retry-after"]
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
}
#[tokio::test]
async fn lembrete_vencido_apos_200_futuros_e_ack_antigo_nao_suprime_reagendamento() {
    let (api, pool, admin, _, admin_id, _) = fixture().await;
    let now = chrono::Utc::now();
    for i in 1..=201 {
        sqlx::query("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em,lembrete_minutos) VALUES(?,'Ainda não vence',?,0)").bind(admin_id).bind((now+chrono::Duration::minutes(i)).to_rfc3339()).execute(&pool).await.unwrap();
    }
    let inicio = (now + chrono::Duration::days(1)).to_rfc3339();
    let id:i64=sqlx::query_scalar("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em,lembrete_minutos) VALUES(?,'Vencido',?,2880) RETURNING id").bind(admin_id).bind(&inicio).fetch_one(&pool).await.unwrap();
    let tasks = api
        .json(
            Method::GET,
            "/api/calendario/lembretes",
            &admin,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["id"], id);
    sqlx::query("UPDATE tarefa_calendario SET inicio_em=? WHERE id=?")
        .bind((now + chrono::Duration::days(2)).to_rfc3339())
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    api.json(Method::PATCH,&format!("/api/calendario/lembretes/{id}/dispensar"),&admin,json!({"inicio_em":inicio,"lembrete_minutos":2880,"data_atualizacao":tasks[0]["data_atualizacao"]}),StatusCode::CONFLICT).await;
    assert!(
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT lembrete_dispensado_em FROM tarefa_calendario WHERE id=?"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn uploads_concorrentes_respeitam_quantidade_e_limite_por_arquivo() {
    let mut config = crate::config::Config::from_env().unwrap();
    config.database_url = "sqlite::memory:".into();
    config.max_upload_bytes = 1024;
    config.admin_login = Some("uploads-test".into());
    config.admin_password = Some("senha-uploads-segura".into());
    let pool = crate::db::conectar(&config).await.unwrap();
    let app = crate::construir_app(crate::AppState {
        pool: pool.clone(),
        config,
        auth_runtime: crate::handlers::auth::AuthRuntime::default(),
        backup_runtime: crate::handlers::backup::BackupRuntime::default(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let api = TestApi {
        client: reqwest::Client::new(),
        base,
        task,
    };
    let login = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"uploads-test","senha":"senha-uploads-segura"}),
            StatusCode::OK,
        )
        .await;
    let token = login["token"].as_str().unwrap();
    let uid = login["usuario"]["id"].as_i64().unwrap();
    let pessoa: i64 = sqlx::query_scalar("INSERT INTO pessoa(nome) VALUES('Arquivo') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let path = format!("/api/dossie/pessoas/{pessoa}/anexos");
    arquivo(
        &api,
        &path,
        token,
        &vec![b'x'; 1025],
        StatusCode::PAYLOAD_TOO_LARGE,
    )
    .await;
    let accepted = arquivo(&api, &path, token, &vec![b'x'; 1024], StatusCode::CREATED).await;
    assert_eq!(accepted["tamanho_bytes"], 1024);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM anexo_dossie")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    let tid:i64=sqlx::query_scalar("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em) VALUES(?,'Upload','2026-10-07T12:00:00Z') RETURNING id").bind(uid).fetch_one(&pool).await.unwrap();
    for _ in 0..29 {
        sqlx::query("INSERT INTO anexo_tarefa_calendario(tarefa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes) VALUES(?,'teste','text/plain',x'78',1)").bind(tid).execute(&pool).await.unwrap();
    }
    let body=b"--melhoria\r\nContent-Disposition: form-data; name=\"arquivo\"; filename=\"teste.txt\"\r\n\r\nx\r\n--melhoria--\r\n";
    let request = || {
        api.client
            .post(format!("{}/api/calendario/tarefas/{tid}/anexos", api.base))
            .bearer_auth(token)
            .header("content-type", "multipart/form-data; boundary=melhoria")
            .body(body.to_vec())
            .send()
    };
    let (a, b) = tokio::join!(request(), request());
    let mut statuses = vec![a.unwrap().status().as_u16(), b.unwrap().status().as_u16()];
    statuses.sort();
    assert_eq!(statuses, vec![201, 400]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM anexo_tarefa_calendario WHERE tarefa_id=?"
        )
        .bind(tid)
        .fetch_one(&pool)
        .await
        .unwrap(),
        30
    );
}
