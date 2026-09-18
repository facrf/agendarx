use crate::integration_tests::TestApi;
use crate::{AppState, config::Config, construir_app, db, handlers::backup::BackupRuntime};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};

#[tokio::test]
async fn hp_previa_revisoes_historico_e_atomicidade() {
    let mut config = Config::from_env().unwrap();
    config.database_url = "sqlite::memory:".into();
    config.admin_login = Some("admin-revisao".into());
    config.admin_password = Some("senha-admin-revisao".into());
    let pool = db::conectar(&config).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = construir_app(AppState {
        pool: pool.clone(),
        config,
        backup_runtime: BackupRuntime::default(),
    });
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let api = TestApi {
        client: Client::new(),
        base,
        task,
    };
    let login = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"admin-revisao","senha":"senha-admin-revisao"}),
            StatusCode::OK,
        )
        .await;
    let token = login["token"].as_str().unwrap();
    api.json(
        Method::POST,
        "/api/pessoas/risco/previa",
        "",
        json!({"classificacao_risco":"SEM_RISCO","toxicidade":0}),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    let source = api.json(Method::POST, "/api/pessoas", token, json!({"nome":"Fonte","classificacao_risco":"MANIPULATIVO","toxicidade":0.5,"risco_justificativa":"Observações iniciais","risco_revisado_em":"2026-09-17"}), StatusCode::CREATED).await;
    let a = source["id"].as_i64().unwrap();
    assert_eq!(
        source["risco_registro"]["justificativa"],
        "Observações iniciais"
    );
    let b = api
        .json(
            Method::POST,
            "/api/pessoas",
            token,
            json!({"nome":"Alvo"}),
            StatusCode::CREATED,
        )
        .await["id"]
        .as_i64()
        .unwrap();
    let c = api
        .json(
            Method::POST,
            "/api/pessoas",
            token,
            json!({"nome":"Segundo grau"}),
            StatusCode::CREATED,
        )
        .await["id"]
        .as_i64()
        .unwrap();
    for (origem, destino) in [(a, b), (b, c)] {
        api.json(
            Method::POST,
            "/api/vinculos",
            token,
            json!({"pessoa_origem_id":origem,"pessoa_destino_id":destino,"tipo_vinculo":"Família"}),
            StatusCode::CREATED,
        )
        .await;
    }
    let history_path = format!("/api/pessoas/{a}/risco/historico");
    let history = api
        .json(
            Method::GET,
            &history_path,
            token,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["anterior"], Value::Null);
    assert_eq!(history[0]["autor_login"], "admin-revisao");
    let preview = api
        .json(
            Method::POST,
            "/api/pessoas/risco/previa",
            token,
            json!({"pessoa_id":a,"classificacao_risco":"MANIPULATIVO","toxicidade":0.3}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(preview["alteracoes"].as_array().unwrap().len(), 3);
    assert_eq!(preview["pessoa"]["hp"], 0.7);
    assert_eq!(preview["pessoa"]["penalidade_propria"], 0.3);
    let unchanged = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{a}"),
            token,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(unchanged["toxicidade"], 0.5);
    assert_eq!(
        api.json(
            Method::GET,
            &history_path,
            token,
            Value::Null,
            StatusCode::OK
        )
        .await,
        history
    );
    let update = json!({"nome":"Fonte","classificacao_risco":"MANIPULATIVO","toxicidade":0.3,"risco_justificativa":"Revisão contextualizada","risco_revisado_em":"2026-09-18"});
    let saved = api
        .json(
            Method::PUT,
            &format!("/api/pessoas/{a}"),
            token,
            update.clone(),
            StatusCode::OK,
        )
        .await;
    assert_eq!(saved["psicossocial"], preview["pessoa"]);
    let graph = api
        .json(
            Method::GET,
            "/api/vinculos/grafo",
            token,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    for change in preview["alteracoes"].as_array().unwrap() {
        let node = graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == change["pessoa_id"])
            .unwrap();
        assert_eq!(node["hp"], change["hp_depois"]);
    }
    let history = api
        .json(
            Method::GET,
            &history_path,
            token,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(history.as_array().unwrap().len(), 2);
    assert_eq!(history[0]["anterior"]["toxicidade"], 0.5);
    assert_eq!(history[0]["novo"]["toxicidade"], 0.3);
    assert_eq!(
        history[0]["novo"]["justificativa"],
        "Revisão contextualizada"
    );
    for body in [update.clone(), json!({"nome":"Fonte renomeada"})] {
        api.json(
            Method::PUT,
            &format!("/api/pessoas/{a}"),
            token,
            body,
            StatusCode::OK,
        )
        .await;
    }
    assert_eq!(
        api.json(
            Method::GET,
            &history_path,
            token,
            Value::Null,
            StatusCode::OK
        )
        .await,
        history
    );
    for (field, value) in [
        ("risco_revisado_em", json!("2026-02-30")),
        ("risco_justificativa", json!("x".repeat(5001))),
        (
            "contatos",
            json!([{"id":999,"tipo_contato_id":1,"valor":"inválido"}]),
        ),
    ] {
        let mut invalid = update.clone();
        invalid["toxicidade"] = json!(0.4);
        invalid[field] = value;
        api.json(
            Method::PUT,
            &format!("/api/pessoas/{a}"),
            token,
            invalid,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(
            api.json(
                Method::GET,
                &history_path,
                token,
                Value::Null,
                StatusCode::OK
            )
            .await,
            history
        );
        assert_eq!(
            api.json(
                Method::GET,
                &format!("/api/pessoas/{a}"),
                token,
                Value::Null,
                StatusCode::OK
            )
            .await["toxicidade"],
            0.3
        );
    }
    api.json(
        Method::POST,
        "/api/pessoas/risco/previa",
        token,
        json!({"pessoa_id":999,"classificacao_risco":"MANIPULATIVO","toxicidade":0.3}),
        StatusCode::NOT_FOUND,
    )
    .await;
    api.json(
        Method::POST,
        "/api/pessoas/risco/previa",
        token,
        json!({"pessoa_id":a,"classificacao_risco":"MANIPULATIVO","toxicidade":0.9}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let new_preview = api
        .json(
            Method::POST,
            "/api/pessoas/risco/previa",
            token,
            json!({"nome":"Nova pessoa","classificacao_risco":"MANIPULATIVO","toxicidade":0.4}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(new_preview["pessoa"]["hp"], 0.6);
    assert_eq!(new_preview["alteracoes"].as_array().unwrap().len(), 1);
    assert_eq!(new_preview["alteracoes"][0]["hp_antes"], Value::Null);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pessoa")
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
    api.json(
        Method::PUT,
        &format!("/api/pessoas/{a}"),
        token,
        json!({"nome":"Fonte","risco_justificativa":"","risco_revisado_em":""}),
        StatusCode::OK,
    )
    .await;
    let cleared = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{a}"),
            token,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(cleared["risco_registro"]["justificativa"], "");
    assert_eq!(cleared["risco_registro"]["revisado_em"], Value::Null);
    api.json(
        Method::DELETE,
        &format!("/api/pessoas/{a}"),
        token,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    api.json(
        Method::GET,
        &history_path,
        token,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    api.json(
        Method::POST,
        &format!("/api/produtividade/lixeira/{a}/restaurar"),
        token,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        api.json(
            Method::GET,
            &history_path,
            token,
            Value::Null,
            StatusCode::OK
        )
        .await
        .as_array()
        .unwrap()
        .len(),
        3
    );
}

#[tokio::test]
async fn hp_snapshot_cadastro_configuracao_permissoes_e_lixeira() {
    let mut config = Config::from_env().unwrap();
    config.database_url = "sqlite::memory:".into();
    config.admin_login = Some("admin-hp".into());
    config.admin_password = Some("senha-admin-hp".into());
    let pool = db::conectar(&config).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = construir_app(AppState {
        pool: pool.clone(),
        config,
        backup_runtime: BackupRuntime::default(),
    });
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let api = TestApi {
        client: Client::new(),
        base,
        task,
    };
    let login = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"admin-hp","senha":"senha-admin-hp"}),
            StatusCode::OK,
        )
        .await;
    let admin = login["token"].as_str().unwrap();
    api.json(
        Method::POST,
        "/api/configuracoes/admin/usuarios",
        admin,
        json!({"login":"usuario-hp","senha":"senha-usuario-hp","perfil":"usuario"}),
        StatusCode::CREATED,
    )
    .await;
    let login = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"usuario-hp","senha":"senha-usuario-hp"}),
            StatusCode::OK,
        )
        .await;
    let user = login["token"].as_str().unwrap();
    let cfg = api
        .json(
            Method::GET,
            "/api/configuracoes/hp-psicossocial",
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(cfg["versao"], 1);
    let mut payload = cfg.clone();
    payload["versao_esperada"] = json!(1);
    api.json(
        Method::PUT,
        "/api/configuracoes/hp-psicossocial",
        user,
        payload.clone(),
        StatusCode::FORBIDDEN,
    )
    .await;
    api.json(
        Method::POST,
        "/api/pessoas",
        user,
        json!({"nome":"Inválido","classificacao_risco":"MANIPULATIVO","toxicidade":0.51}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    api.json(
        Method::POST,
        "/api/pessoas",
        user,
        json!({"nome":"Incompleto","toxicidade":0.3}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let source = api
        .json(
            Method::POST,
            "/api/pessoas",
            user,
            json!({"nome":"Fonte","classificacao_risco":"MANIPULATIVO","toxicidade":0.5}),
            StatusCode::CREATED,
        )
        .await;
    let a = source["id"].as_i64().unwrap();
    assert_eq!(source["psicossocial"]["aura_nome"], "Crítico");
    assert_eq!(source["psicossocial"]["hp"], 0.5);
    assert_eq!(source["psicossocial"]["penalidade_propria"], 0.5);
    let b = api
        .json(
            Method::POST,
            "/api/pessoas",
            user,
            json!({"nome":"Alvo"}),
            StatusCode::CREATED,
        )
        .await["id"]
        .as_i64()
        .unwrap();
    let c = api
        .json(
            Method::POST,
            "/api/pessoas",
            user,
            json!({"nome":"Segundo grau"}),
            StatusCode::CREATED,
        )
        .await["id"]
        .as_i64()
        .unwrap();
    let edge = api
        .json(
            Method::POST,
            "/api/vinculos",
            user,
            json!({"pessoa_origem_id":a,"pessoa_destino_id":b,"tipo_vinculo":"Família"}),
            StatusCode::CREATED,
        )
        .await;
    api.json(
        Method::POST,
        "/api/vinculos",
        user,
        json!({"pessoa_origem_id":c,"pessoa_destino_id":b,"tipo_vinculo":"Profissional"}),
        StatusCode::CREATED,
    )
    .await;
    let graph = api
        .json(
            Method::GET,
            "/api/vinculos/grafo",
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    let node = |id| {
        graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == id)
            .unwrap()
    };
    assert_eq!(node(b)["hp"], 0.5);
    assert_eq!(node(b)["aura_nome"], "Estável");
    assert_eq!(node(c)["hp"], 0.9);
    assert_eq!(node(c)["contribuicoes"][0]["grau"], 2);
    let profile = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{c}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(profile["psicossocial"]["hp"], node(c)["hp"]);
    assert_eq!(
        profile["psicossocial"]["contribuicoes"],
        node(c)["contribuicoes"]
    );
    api.json(
        Method::PUT,
        &format!("/api/pessoas/{a}"),
        user,
        json!({"nome":"Fonte renomeada"}),
        StatusCode::OK,
    )
    .await;
    let source = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{a}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(source["toxicidade"], 0.5);
    payload["toxicidade_max"] = json!(0.4);
    api.json(
        Method::PUT,
        "/api/configuracoes/hp-psicossocial",
        admin,
        payload.clone(),
        StatusCode::CONFLICT,
    )
    .await;
    payload["toxicidade_max"] = json!(0.5);
    payload["fator_segundo_grau"] = json!(0.4);
    let updated = api
        .json(
            Method::PUT,
            "/api/configuracoes/hp-psicossocial",
            admin,
            payload.clone(),
            StatusCode::OK,
        )
        .await;
    assert_eq!(updated["versao"], 2);
    api.json(
        Method::PUT,
        "/api/configuracoes/hp-psicossocial",
        admin,
        payload,
        StatusCode::CONFLICT,
    )
    .await;
    let profile = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{c}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(profile["psicossocial"]["hp"], 0.8);
    api.json(
        Method::DELETE,
        &format!("/api/pessoas/{a}"),
        user,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    let profile = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{c}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(profile["psicossocial"]["hp"], 1.0);
    api.json(
        Method::POST,
        &format!("/api/produtividade/lixeira/{a}/restaurar"),
        admin,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    let profile = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{c}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(profile["psicossocial"]["hp"], 0.8);
    api.json(
        Method::DELETE,
        &format!("/api/vinculos/{}", edge["id"]),
        user,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    let profile = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{c}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(profile["psicossocial"]["hp"], 1.0);
    let row: (i64, String) = sqlx::query_as(
        "SELECT versao, parametros_json FROM hp_psicossocial_configuracao WHERE id = 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, 2);
    assert_eq!(
        serde_json::from_str::<Value>(&row.1).unwrap()["fator_segundo_grau"],
        0.4
    );
}
