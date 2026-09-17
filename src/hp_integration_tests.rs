use crate::integration_tests::TestApi;
use crate::{AppState, config::Config, construir_app, db, handlers::backup::BackupRuntime};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};

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
    assert_eq!(source["psicossocial"]["hp"], 1.0);
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
