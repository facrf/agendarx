use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};

use crate::{AppState, config::Config, construir_app, db};

struct TestApi {
    client: Client,
    base: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestApi {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestApi {
    async fn json(
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
