use std::{str::FromStr, time::Duration};

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::{config::Config, error::AppError};

pub async fn conectar(config: &Config) -> Result<SqlitePool, AppError> {
    criar_diretorio_do_banco(&config.database_url)?;

    let opcoes = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opcoes)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    criar_admin_inicial(&pool, config).await?;

    Ok(pool)
}

fn criar_diretorio_do_banco(database_url: &str) -> Result<(), AppError> {
    let Some(caminho) = database_url
        .strip_prefix("sqlite://")
        .and_then(|valor| valor.split('?').next())
    else {
        return Ok(());
    };

    let caminho = std::path::Path::new(caminho);
    if let Some(diretorio) = caminho.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(diretorio)?;
    }
    Ok(())
}

async fn criar_admin_inicial(pool: &SqlitePool, config: &Config) -> Result<(), AppError> {
    let (Some(login), Some(senha)) = (&config.admin_login, &config.admin_password) else {
        return Ok(());
    };

    let existe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM usuario WHERE login = ?)")
        .bind(login)
        .fetch_one(pool)
        .await?;
    if existe {
        return Ok(());
    }

    let senha = senha.clone();
    let senha_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(senha.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::interno("falha ao gerar hash da senha"))?
    .map_err(|_| AppError::interno("falha ao gerar hash da senha"))?;

    sqlx::query("INSERT INTO usuario (login, senha_hash) VALUES (?, ?)")
        .bind(login)
        .bind(senha_hash)
        .execute(pool)
        .await?;
    tracing::info!(login, "usuário administrador inicial criado");
    Ok(())
}
