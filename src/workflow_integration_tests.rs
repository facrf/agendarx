use crate::{
    AppState, config::Config, construir_app, db, handlers::backup::BackupRuntime,
    integration_tests::TestApi,
};
use chrono::{Duration, SecondsFormat, Utc};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};

pub(crate) async fn fixture() -> (TestApi, sqlx::SqlitePool, String, String, i64, i64) {
    let mut config = Config::from_env().unwrap();
    config.database_url = "sqlite::memory:".into();
    config.admin_login = Some("admin-fluxos".into());
    config.admin_password = Some("senha-fluxos-segura".into());
    let pool = db::conectar(&config).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = construir_app(AppState {
        pool: pool.clone(),
        config,
        auth_runtime: crate::handlers::auth::AuthRuntime::default(),
        backup_runtime: BackupRuntime::default(),
    });
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let api = TestApi {
        client: Client::new(),
        base,
        task,
    };
    let admin = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"admin-fluxos","senha":"senha-fluxos-segura"}),
            StatusCode::OK,
        )
        .await;
    let admin_id = admin["usuario"]["id"].as_i64().unwrap();
    let admin = admin["token"].as_str().unwrap().to_owned();
    api.json(
        Method::POST,
        "/api/configuracoes/admin/usuarios",
        &admin,
        json!({"login":"usuario-fluxos","senha":"senha-fluxos-segura","perfil":"usuario"}),
        StatusCode::CREATED,
    )
    .await;
    let user = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"usuario-fluxos","senha":"senha-fluxos-segura"}),
            StatusCode::OK,
        )
        .await;
    let user_id = user["usuario"]["id"].as_i64().unwrap();
    let user = user["token"].as_str().unwrap().to_owned();
    (api, pool, admin, user, admin_id, user_id)
}
async fn get(api: &TestApi, path: &str, token: &str) -> Value {
    api.json(Method::GET, path, token, Value::Null, StatusCode::OK)
        .await
}

#[tokio::test]
async fn busca_indexada_pagina_atualiza_e_isola_tarefas() {
    let (api, pool, admin, user, admin_id, user_id) = fixture().await;
    for i in 0..43 {
        sqlx::query(
            "INSERT INTO pessoa(nome,descricao,categoria_id) VALUES(?, 'Reunião comercial',1)",
        )
        .bind(format!("Contato {i:02}"))
        .execute(&pool)
        .await
        .unwrap();
    }
    let page = get(
        &api,
        "/api/pessoas/paginadas?busca=reuniao&por_pagina=10&pagina=2&categoria=1&tipo=fisica",
        &admin,
    )
    .await;
    assert_eq!(page["total"], 43);
    assert_eq!(page["itens"].as_array().unwrap().len(), 10);
    assert_eq!(page["itens"][0]["nome"], "Contato 10");
    assert_eq!(page["total_paginas"], 5);
    sqlx::query("INSERT INTO pessoa_favorita(usuario_id,pessoa_id) VALUES(?,1)")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(&api, "/api/pessoas/paginadas?favoritos=true", &user).await["total"],
        1
    );
    assert_eq!(
        get(&api, "/api/pessoas/paginadas?favoritos=true", &admin).await["total"],
        0
    );
    sqlx::query(
        "INSERT INTO contato(pessoa_id,tipo_contato_id,valor) VALUES(1,1,'exclusivocontato')",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        get(
            &api,
            "/api/pessoas/paginadas?busca=exclusivocontato",
            &admin
        )
        .await["total"],
        1
    );
    sqlx::query("UPDATE contato SET valor='alteradocontato' WHERE pessoa_id=1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(
            &api,
            "/api/pessoas/paginadas?busca=exclusivocontato",
            &admin
        )
        .await["total"],
        0
    );
    let task:i64=sqlx::query_scalar("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em) VALUES(?,'segredotarefa','2026-10-06T10:00:00.000Z') RETURNING id").bind(user_id).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO tarefa_calendario_pessoa(tarefa_id,pessoa_id) VALUES(?,1)")
        .bind(task)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(&api, "/api/busca?busca=segredo", &admin).await["total"],
        0
    );
    assert_eq!(
        get(&api, "/api/busca?busca=segredo", &user).await["total"],
        1
    );
    assert_eq!(
        get(&api, "/api/pessoas/paginadas?busca=segredo", &admin).await["total"],
        0
    );
    assert_eq!(
        get(&api, "/api/pessoas/paginadas?busca=segredo", &user).await["total"],
        1
    );
    sqlx::query("INSERT INTO anexo_dossie(pessoa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) VALUES(1,'registro.txt','text/plain',?,10,'notaparticular')").bind(b"arquivoindexado".to_vec()).execute(&pool).await.unwrap();
    assert_eq!(
        get(&api, "/api/pessoas/paginadas?busca=arquivoindexado", &admin).await["total"],
        1
    );
    assert_eq!(
        get(&api, "/api/busca?busca=notaparticular&tipo=anexo", &admin).await["total"],
        1
    );
    sqlx::query("UPDATE pessoa SET excluida_em=CURRENT_TIMESTAMP WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(&api, "/api/busca?busca=arquivoindexado", &admin).await["total"],
        0
    );
    sqlx::query("UPDATE pessoa SET excluida_em=NULL WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(&api, "/api/busca?busca=arquivoindexado", &admin).await["total"],
        1
    );
    assert_eq!(
        get(&api, "/api/busca?busca=%22%2A%29%20OR%20%28%22", &admin).await["total"],
        0
    );
    api.json(
        Method::GET,
        "/api/pessoas/paginadas?pagina=0",
        &admin,
        Value::Null,
        StatusCode::BAD_REQUEST,
    )
    .await;

    sqlx::query("INSERT INTO anexo_tarefa_calendario(tarefa_id,nome_arquivo,mime_type,conteudo_blob,tamanho_bytes,notas) VALUES(?,'privado.txt','text/plain',?,13,'')").bind(task).bind(b"arquivoprivado".to_vec()).execute(&pool).await.unwrap();
    assert_eq!(
        get(&api, "/api/busca?busca=arquivoprivado", &admin).await["total"],
        0
    );
    assert_eq!(
        get(&api, "/api/busca?busca=arquivoprivado", &user).await["total"],
        1
    );
    assert_eq!(
        get(&api, "/api/pessoas/paginadas?busca=arquivoprivado", &admin).await["total"],
        0
    );
    sqlx::query("INSERT INTO pessoa_vinculo(pessoa_origem_id,pessoa_destino_id,tipo_vinculo,descricao) VALUES(1,2,'Parceria','conversaindexada')").execute(&pool).await.unwrap();
    sqlx::query("UPDATE pessoa SET nome='Renomeadopessoa' WHERE id=2")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(
            &api,
            "/api/busca?busca=renomeadopessoa&tipo=vinculo",
            &admin
        )
        .await["total"],
        1
    );
    assert_eq!(
        get(
            &api,
            "/api/pessoas/paginadas?busca=conversaindexada",
            &admin
        )
        .await["total"],
        2
    );
    sqlx::query("UPDATE pessoa_vinculo SET excluido_em=CURRENT_TIMESTAMP WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(&api, "/api/busca?busca=conversaindexada", &admin).await["total"],
        0
    );
    sqlx::query("INSERT INTO etiqueta(nome) VALUES('etiquetaindexada')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO pessoa_etiqueta(pessoa_id,etiqueta_id) VALUES(3,1)")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(
            &api,
            "/api/pessoas/paginadas?busca=etiquetaindexada",
            &admin
        )
        .await["total"],
        1
    );
    sqlx::query("DELETE FROM etiqueta WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(
            &api,
            "/api/pessoas/paginadas?busca=etiquetaindexada",
            &admin
        )
        .await["total"],
        0
    );
    let _ = admin_id;
}

#[tokio::test]
async fn painel_auditoria_e_posicoes_respeitam_conta_e_filtros() {
    let (api, pool, admin, user, admin_id, user_id) = fixture().await;
    let now = Utc::now();
    let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = start + Duration::days(1);
    for (owner, title, time, status) in [
        (admin_id, "Hoje", now, "PENDENTE"),
        (admin_id, "Concluída", now, "CONCLUIDA"),
        (admin_id, "Atrasada", start - Duration::days(2), "PENDENTE"),
        (admin_id, "Próxima", end + Duration::hours(1), "PENDENTE"),
        (user_id, "Privada", now, "PENDENTE"),
    ] {
        sqlx::query(
            "INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em,status) VALUES(?,?,?,?)",
        )
        .bind(owner)
        .bind(title)
        .bind(time.to_rfc3339_opts(SecondsFormat::Millis, true))
        .bind(status)
        .execute(&pool)
        .await
        .unwrap();
    }
    let path = format!(
        "/api/painel?inicio={}&fim={}",
        start.to_rfc3339_opts(SecondsFormat::Millis, true),
        end.to_rfc3339_opts(SecondsFormat::Millis, true)
    );
    let dashboard = get(&api, &path, &admin).await;
    assert_eq!(dashboard["total_hoje"], 1);
    assert_eq!(dashboard["total_atrasadas"], 1);
    assert_eq!(dashboard["total_proximas"], 1);
    assert_eq!(
        get(&api, &path, &user).await["hoje"][0]["titulo"],
        "Privada"
    );

    for i in 0..25 {
        sqlx::query("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em) VALUES(?,?,?)")
            .bind(admin_id)
            .bind(format!("Pendência {i}"))
            .bind(now.to_rfc3339_opts(SecondsFormat::Millis, true))
            .execute(&pool)
            .await
            .unwrap();
    }
    let larger = get(&api, &path, &admin).await;
    assert_eq!(larger["total_hoje"], 26);
    assert_eq!(larger["hoje"].as_array().unwrap().len(), 20);
    // A local day starting at 03:00 UTC still includes its all-day task at 00:00 UTC.
    sqlx::query("INSERT INTO tarefa_calendario(usuario_id,titulo,inicio_em,dia_inteiro) VALUES(?,'Dia inteiro local',?,1)").bind(user_id).bind(start.to_rfc3339_opts(SecondsFormat::Millis,true)).execute(&pool).await.unwrap();
    let local_path = format!(
        "/api/painel?inicio={}&fim={}&dia={}",
        (start + Duration::hours(3)).to_rfc3339_opts(SecondsFormat::Millis, true),
        (end + Duration::hours(3)).to_rfc3339_opts(SecondsFormat::Millis, true),
        start.date_naive()
    );
    let local = get(&api, &local_path, &user).await;
    assert!(
        local["hoje"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["titulo"] == "Dia inteiro local")
    );
    sqlx::query("INSERT INTO pessoa(nome) VALUES('Mapa')")
        .execute(&pool)
        .await
        .unwrap();
    api.json(
        Method::PUT,
        "/api/produtividade/grafo/posicoes/force",
        &admin,
        json!([{"pessoa_id":1,"x":10,"y":20}]),
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        get(&api, "/api/produtividade/grafo/posicoes/force", &admin).await[0]["x"],
        10.0
    );
    assert_eq!(
        get(&api, "/api/produtividade/grafo/posicoes/force", &user).await,
        json!([])
    );
    api.json(
        Method::PUT,
        "/api/produtividade/grafo/posicoes/force",
        &admin,
        json!([{"pessoa_id":999,"x":10,"y":20}]),
        StatusCode::NOT_FOUND,
    )
    .await;
    api.json(
        Method::PUT,
        "/api/produtividade/grafo/posicoes/force",
        &admin,
        json!([{"pessoa_id":1,"x":1e10,"y":20}]),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let audit=get(&api,"/api/produtividade/auditoria/paginada?usuario=admin-fluxos&recurso=grafo&acao=ALTERAR&status=204",&admin).await;
    api.json(
        Method::PUT,
        "/api/produtividade/grafo/posicoes/force",
        &admin,
        json!([{"pessoa_id":1,"x":30,"y":40},{"pessoa_id":999,"x":10,"y":20}]),
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(
        get(&api, "/api/produtividade/grafo/posicoes/force", &admin).await[0]["x"],
        10.0
    );
    assert_eq!(audit["total"], 1);
    assert_eq!(audit["itens"][0]["metodo"], "PUT");
    assert!(audit["itens"][0]["duracao_ms"].is_number());
    api.json(
        Method::GET,
        "/api/produtividade/auditoria/paginada",
        &user,
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
    api.json(
        Method::GET,
        "/api/produtividade/auditoria/paginada?desde=2026-10-08&ate=2026-10-01",
        &admin,
        Value::Null,
        StatusCode::BAD_REQUEST,
    )
    .await;
}
