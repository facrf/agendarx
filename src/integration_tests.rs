use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};

use crate::{AppState, config::Config, construir_app, db, handlers::backup::BackupRuntime};

pub(crate) struct TestApi {
    pub(crate) client: Client,
    pub(crate) base: String,
    pub(crate) task: tokio::task::JoinHandle<()>,
}

impl Drop for TestApi {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestApi {
    pub(crate) async fn json(
        &self,
        method: Method,
        path: &str,
        token: &str,
        body: Value,
        status: StatusCode,
    ) -> Value {
        let response = self
            .client
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let actual = response.status();
        let text = response.text().await.unwrap();
        assert_eq!(actual, status, "{path}: {text}");
        if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap()
        }
    }

    async fn upload(&self, path: &str, token: &str, bytes: &[u8], status: StatusCode) -> Value {
        let mut body = b"--agendarx-test\r\nContent-Disposition: form-data; name=\"arquivo\"; filename=\"teste.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n".to_vec();
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n--agendarx-test--\r\n");
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .bearer_auth(token)
            .header(
                "content-type",
                "multipart/form-data; boundary=agendarx-test",
            )
            .body(body)
            .send()
            .await
            .unwrap();
        let actual = response.status();
        let text = response.text().await.unwrap();
        assert_eq!(actual, status, "{path}: {text}");
        serde_json::from_str(&text).unwrap_or(Value::Null)
    }
}

#[tokio::test]
async fn rotas_api_inexistentes_retornam_json_e_preservam_frontend() {
    let mut config = Config::from_env().unwrap();
    config.frontend_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("frontend");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_lazy("sqlite::memory:")
        .unwrap();
    let app = construir_app(AppState {
        pool,
        config,
        backup_runtime: BackupRuntime::default(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let api = TestApi {
        client: Client::new(),
        base,
        task,
    };

    for path in [
        "/api",
        "/api/",
        "/api/inexistente",
        "/api/configuracoes/inexistente",
        "/api/auth/inexistente",
    ] {
        for method in [Method::GET, Method::POST] {
            let response = api
                .client
                .request(method, format!("{}{path}", api.base))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(response.headers()["content-type"], "application/json");
            assert_eq!(
                response.json::<Value>().await.unwrap(),
                json!({"erro": "Rota da API não encontrada"})
            );
        }
    }
    for path in ["/", "/configuracoes", "/api-exemplo"] {
        let response = api
            .client
            .get(format!("{}{path}", api.base))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert!(response.text().await.unwrap().contains("<!doctype html>"));
    }
    let response = api
        .client
        .get(format!("{}/api/configuracoes/categorias", api.base))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn uploads_persistencia_transacoes_e_permissoes() {
    let mut config = Config::from_env().unwrap();
    config.database_url = "sqlite::memory:".to_owned();
    config.admin_login = Some("admin-teste".to_owned());
    config.admin_password = Some("senha-teste-segura".to_owned());
    config.max_upload_bytes = 4 * 1024 * 1024;
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
            json!({"login":"admin-teste", "senha":"senha-teste-segura"}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(login["usuario"]["perfil"], "admin");
    let admin = login["token"].as_str().unwrap();

    for role in ["usuario", "admin"] {
        let account = api
            .json(
                Method::POST,
                "/api/configuracoes/admin/usuarios",
                admin,
                json!({"login":format!("novo-{role}"), "senha":"senha-segura", "perfil":role}),
                StatusCode::CREATED,
            )
            .await;
        assert_eq!(account["perfil"], role);
        assert!(account.get("senha_hash").is_none());
    }
    api.json(
        Method::POST,
        "/api/configuracoes/admin/usuarios",
        admin,
        json!({"login":"novo-admin", "senha":"senha-segura", "perfil":"admin"}),
        StatusCode::CONFLICT,
    )
    .await;
    for body in [
        json!({"login":"teste", "senha":"curta", "perfil":"usuario"}),
        json!({"login":"teste", "senha":"senha-segura", "perfil":"root"}),
    ] {
        api.json(
            Method::POST,
            "/api/configuracoes/admin/usuarios",
            admin,
            body,
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    let user_login = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"novo-usuario", "senha":"senha-segura"}),
            StatusCode::OK,
        )
        .await;
    let user = user_login["token"].as_str().unwrap();
    for path in [
        "/api/configuracoes/admin/usuarios",
        "/api/configuracoes/admin/diagnostico-armazenamento",
    ] {
        api.json(Method::GET, path, user, Value::Null, StatusCode::FORBIDDEN)
            .await;
    }
    api.json(
        Method::POST,
        "/api/configuracoes/admin/usuarios",
        user,
        json!({"login":"invasor", "senha":"senha-segura", "perfil":"admin"}),
        StatusCode::FORBIDDEN,
    )
    .await;
    api.json(
        Method::POST,
        "/api/configuracoes/categorias",
        user,
        json!({"nome_categoria":"Teste", "cor_hex":"#112233"}),
        StatusCode::FORBIDDEN,
    )
    .await;
    api.json(
        Method::GET,
        "/api/configuracoes/categorias",
        user,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    let second_admin = api
        .json(
            Method::POST,
            "/api/auth/login",
            "",
            json!({"login":"novo-admin", "senha":"senha-segura"}),
            StatusCode::OK,
        )
        .await;
    api.json(
        Method::GET,
        "/api/configuracoes/admin/usuarios",
        second_admin["token"].as_str().unwrap(),
        Value::Null,
        StatusCode::OK,
    )
    .await;

    let person = api
        .json(
            Method::POST,
            "/api/pessoas",
            user,
            json!({"nome":"Pessoa teste", "contatos":[]}),
            StatusCode::CREATED,
        )
        .await;
    let id = person["id"].as_i64().unwrap();
    api.json(
        Method::POST,
        "/api/produtividade/etiquetas",
        user,
        json!({"nome":"Bloqueada","cor_hex":"#112233"}),
        StatusCode::FORBIDDEN,
    )
    .await;
    let tag = api
        .json(
            Method::POST,
            "/api/produtividade/etiquetas",
            admin,
            json!({"nome":"Urgente","cor_hex":"#112233"}),
            StatusCode::CREATED,
        )
        .await;
    api.json(
        Method::PUT,
        &format!("/api/produtividade/pessoas/{id}/etiquetas"),
        user,
        json!({"etiquetas_ids":[tag["id"]]}),
        StatusCode::OK,
    )
    .await;
    api.json(
        Method::PUT,
        &format!("/api/produtividade/pessoas/{id}/favorito"),
        user,
        json!({"favorito":true}),
        StatusCode::NO_CONTENT,
    )
    .await;
    let indexed = api
        .json(
            Method::GET,
            "/api/pessoas",
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(indexed[0]["etiquetas"], "Urgente");
    assert_eq!(indexed[0]["favorito"], true);
    let other_user_view = api
        .json(
            Method::GET,
            "/api/pessoas",
            admin,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(other_user_view[0]["favorito"], false);
    api.json(
        Method::PUT,
        "/api/produtividade/grafo/posicoes/force",
        user,
        json!([{"pessoa_id":id,"x":10.5,"y":20.25}]),
        StatusCode::NO_CONTENT,
    )
    .await;
    let positions = api
        .json(
            Method::GET,
            "/api/produtividade/grafo/posicoes/force",
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(positions[0]["x"], 10.5);
    assert_eq!(
        api.json(
            Method::GET,
            "/api/produtividade/grafo/posicoes/force",
            admin,
            Value::Null,
            StatusCode::OK
        )
        .await,
        json!([])
    );
    api.json(
        Method::DELETE,
        &format!("/api/pessoas/{id}"),
        user,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        api.json(
            Method::GET,
            "/api/pessoas",
            user,
            Value::Null,
            StatusCode::OK
        )
        .await,
        json!([])
    );
    assert_eq!(
        api.json(
            Method::GET,
            "/api/produtividade/lixeira",
            user,
            Value::Null,
            StatusCode::OK
        )
        .await[0]["id"],
        id
    );
    api.json(
        Method::POST,
        &format!("/api/produtividade/lixeira/{id}/restaurar"),
        user,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        api.json(
            Method::GET,
            "/api/pessoas",
            user,
            Value::Null,
            StatusCode::OK
        )
        .await[0]["id"],
        id
    );
    let audit = api
        .json(
            Method::GET,
            "/api/produtividade/auditoria",
            admin,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert!(
        audit
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["usuario_login"] == "novo-usuario"
                && item["recurso"] == format!("/api/pessoas/{id}"))
    );
    api.json(
        Method::GET,
        "/api/produtividade/auditoria",
        user,
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
    let path = format!("/api/dossie/pessoas/{id}");
    let mut image = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1024, 1024)
        .write_to(&mut image, image::ImageFormat::Bmp)
        .unwrap();
    let image = image.into_inner();
    assert!(image.len() > 2 * 1024 * 1024);
    let photo = api
        .client
        .put(format!("{}{path}/foto", api.base))
        .bearer_auth(user)
        .body(image.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(photo.status(), StatusCode::OK);
    api.upload(&format!("{path}/foto"), user, &image, StatusCode::OK)
        .await;
    api.upload(
        &format!("{path}/foto"),
        user,
        b"invalida",
        StatusCode::BAD_REQUEST,
    )
    .await;
    api.upload(
        &format!("{path}/foto"),
        user,
        &vec![7; 4 * 1024 * 1024 + 1],
        StatusCode::PAYLOAD_TOO_LARGE,
    )
    .await;
    let rejected_photo = api
        .client
        .put(format!("{}{path}/foto", api.base))
        .bearer_auth(user)
        .body("não é imagem")
        .send()
        .await
        .unwrap();
    assert_eq!(rejected_photo.status(), StatusCode::BAD_REQUEST);
    let saved = api
        .client
        .get(format!("{}{path}/foto", api.base))
        .bearer_auth(user)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(saved.as_ref(), image);
    let attachment = api
        .upload(&format!("{path}/anexos"), user, &image, StatusCode::CREATED)
        .await;
    let notes_path = format!("/api/dossie/anexos/{}/notas", attachment["id"]);
    let empty = api
        .json(Method::GET, &notes_path, user, Value::Null, StatusCode::OK)
        .await;
    assert_eq!(empty["notas"], "");
    let notes = json!({"notas":"# Origem\n\nFoto da reunião. **Confirmada**."});
    api.json(
        Method::PUT,
        &notes_path,
        user,
        notes.clone(),
        StatusCode::OK,
    )
    .await;
    let saved_notes = api
        .json(Method::GET, &notes_path, user, Value::Null, StatusCode::OK)
        .await;
    assert_eq!(saved_notes["notas"], notes["notas"]);
    assert_eq!(saved_notes["pessoas_ids"], json!([id]));
    let searchable = api
        .json(
            Method::GET,
            "/api/pessoas?busca=reuni%C3%A3o",
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(searchable[0]["id"], id);
    api.json(
        Method::GET,
        &notes_path,
        "",
        Value::Null,
        StatusCode::UNAUTHORIZED,
    )
    .await;
    api.json(
        Method::PUT,
        &notes_path,
        user,
        json!({"notas":"x".repeat(50_001)}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    api.json(
        Method::PUT,
        &notes_path,
        user,
        json!({"notas":""}),
        StatusCode::OK,
    )
    .await;
    api.json(
        Method::PUT,
        "/api/dossie/anexos/999999/notas",
        user,
        notes.clone(),
        StatusCode::NOT_FOUND,
    )
    .await;

    // Calendar notes retain the same ownership checks as attachment downloads.
    let task_id: i64 = sqlx::query_scalar("INSERT INTO tarefa_calendario (usuario_id, titulo, inicio_em) SELECT id, 'Notas', '2026-09-14T10:00:00Z' FROM usuario WHERE login = 'admin-teste' RETURNING id")
        .fetch_one(&pool).await.unwrap();
    let task_attachment = api
        .upload(
            &format!("/api/calendario/tarefas/{task_id}/anexos"),
            admin,
            b"documento",
            StatusCode::CREATED,
        )
        .await;
    let task_notes_path = format!("/api/calendario/anexos/{}/notas", task_attachment["id"]);
    api.json(
        Method::PUT,
        &task_notes_path,
        admin,
        notes.clone(),
        StatusCode::OK,
    )
    .await;
    api.json(
        Method::GET,
        &task_notes_path,
        user,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    api.json(
        Method::PUT,
        &task_notes_path,
        user,
        notes.clone(),
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(
        api.json(
            Method::GET,
            &task_notes_path,
            admin,
            Value::Null,
            StatusCode::OK
        )
        .await["notas"],
        notes["notas"]
    );

    let second_person: i64 =
        sqlx::query_scalar("INSERT INTO pessoa (nome) VALUES ('Vínculo') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let relationship: i64 = sqlx::query_scalar("INSERT INTO pessoa_vinculo (pessoa_origem_id, pessoa_destino_id, tipo_vinculo) VALUES (?, ?, 'Teste') RETURNING id").bind(id).bind(second_person).fetch_one(&pool).await.unwrap();
    let relationship_attachment = api
        .upload(
            &format!("/api/vinculos/{relationship}/anexos"),
            user,
            b"documento",
            StatusCode::CREATED,
        )
        .await;
    let relationship_notes_path = format!(
        "/api/vinculos/anexos/{}/notas",
        relationship_attachment["id"]
    );
    api.json(
        Method::PUT,
        &relationship_notes_path,
        user,
        notes.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        api.json(
            Method::GET,
            &relationship_notes_path,
            user,
            Value::Null,
            StatusCode::OK
        )
        .await["notas"],
        notes["notas"]
    );
    let download = api
        .client
        .get(format!(
            "{}{}",
            api.base,
            attachment["url_download"].as_str().unwrap()
        ))
        .bearer_auth(user)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(download.as_ref(), image);
    let thumbnail = api
        .client
        .get(format!(
            "{}/api/dossie/anexos/{}/thumbnail",
            api.base, attachment["id"]
        ))
        .bearer_auth(user)
        .send()
        .await
        .unwrap();
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()["content-type"], "image/webp");
    let response = api
        .client
        .get(format!("{}{path}/foto", api.base))
        .bearer_auth(user)
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    api.upload(
        &format!("{path}/anexos"),
        user,
        &vec![7; 4 * 1024 * 1024],
        StatusCode::CREATED,
    )
    .await;
    api.upload(
        &format!("{path}/anexos"),
        user,
        &vec![7; 4 * 1024 * 1024 + 1],
        StatusCode::PAYLOAD_TOO_LARGE,
    )
    .await;
    api.upload(
        &format!("{path}/anexos"),
        user,
        &vec![7; 6 * 1024 * 1024],
        StatusCode::PAYLOAD_TOO_LARGE,
    )
    .await;
    api.upload(
        &format!("{path}/anexos"),
        user,
        &[],
        StatusCode::BAD_REQUEST,
    )
    .await;
    let list = api
        .json(
            Method::GET,
            &format!("{path}/anexos"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(list.as_array().unwrap().len(), 2);

    let tipo = api
        .json(
            Method::POST,
            "/api/configuracoes/tipos-contato",
            admin,
            json!({"nome_tipo":"Teste"}),
            StatusCode::CREATED,
        )
        .await;
    let updated = api.json(Method::PUT, &format!("/api/pessoas/{id}"), user,
        json!({"nome":"Editada", "contatos":[{"tipo_contato_id":tipo["id"], "valor":"contato"}]}), StatusCode::OK).await;
    let contato = &updated["contatos"][0];
    api.json(
        Method::PUT,
        &format!("/api/pessoas/{id}"),
        user,
        json!({"nome":"Não salvar", "contatos":[{"tipo_contato_id":999999, "valor":"inválido"}]}),
        StatusCode::CONFLICT,
    )
    .await;
    let unchanged = api
        .json(
            Method::GET,
            &format!("/api/pessoas/{id}"),
            user,
            Value::Null,
            StatusCode::OK,
        )
        .await;
    assert_eq!(unchanged["nome"], "Editada");
    assert_eq!(unchanged["contatos"][0], *contato);
    let edited_contact =
        json!({"id":contato["id"], "tipo_contato_id":tipo["id"], "valor":"alterado"});
    let edited = api
        .json(
            Method::PUT,
            &format!("/api/pessoas/{id}"),
            user,
            json!({"nome":"Editada", "contatos":[edited_contact]}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(edited["contatos"][0]["id"], contato["id"]);
    assert_eq!(edited["contatos"][0]["valor"], "alterado");
    let other = api
        .json(
            Method::POST,
            "/api/pessoas",
            user,
            json!({"nome":"Outra", "contatos":[]}),
            StatusCode::CREATED,
        )
        .await;
    api.json(
        Method::PUT,
        &format!("/api/pessoas/{}", other["id"]),
        user,
        json!({"nome":"Não salvar", "contatos":[contato]}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    api.json(
        Method::PUT,
        &format!("/api/pessoas/{id}"),
        user,
        json!({"nome":"Editada", "contatos":[contato, contato]}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    // O perfil é consultado no banco em cada requisição, sem confiar em tokens antigos.
    sqlx::query("UPDATE usuario SET perfil = 'usuario' WHERE login = 'novo-admin'")
        .execute(&pool)
        .await
        .unwrap();
    api.json(
        Method::GET,
        "/api/configuracoes/admin/usuarios",
        second_admin["token"].as_str().unwrap(),
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
    pool.close().await;
}

#[tokio::test]
async fn migracao_preserva_administradores_existentes() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0001_schema_inicial.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO usuario (login, senha_hash) VALUES ('existente', 'hash-teste')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0012_perfis_usuario.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let perfil: String = sqlx::query_scalar("SELECT perfil FROM usuario WHERE login = 'existente'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(perfil, "admin");
    sqlx::query("INSERT INTO usuario (login, senha_hash) VALUES ('nova', 'hash-teste')")
        .execute(&pool)
        .await
        .unwrap();
    let perfil: String = sqlx::query_scalar("SELECT perfil FROM usuario WHERE login = 'nova'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(perfil, "usuario");
    pool.close().await;
}
