use std::{str::FromStr, time::Duration};

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

use crate::{config::Config, error::AppError};

pub async fn conectar(config: &Config) -> Result<SqlitePool, AppError> {
    criar_diretorio_do_banco(&config.database_url)?;

    let mut opcoes = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    // Modo WAL permite leituras e escritas concorrentes sem contenção de locks de arquivo.
    // Em bancos persistidos em arquivo, synchronous=NORMAL é seguro contra falhas e reduz fsyncs.
    if !config.database_url.contains(":memory:") {
        opcoes = opcoes
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
    }

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

    // As variáveis ADMIN_* servem somente para o bootstrap. Depois que o usuário
    // altera o login pela interface, uma reinicialização não deve recriar a conta
    // antiga configurada no ambiente.
    let existe: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM usuario)")
        .fetch_one(pool)
        .await?;
    if existe {
        return Ok(());
    }

    crate::handlers::auth::validar_novo_login(login)
        .map_err(|erro| AppError::configuracao(format!("ADMIN_LOGIN inválido: {erro}")))?;
    let senha = crate::handlers::auth::validar_nova_senha(senha.clone())
        .map_err(|erro| AppError::configuracao(format!("ADMIN_PASSWORD inválida: {erro}")))?;
    let senha_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(senha.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::interno("falha ao gerar hash da senha"))?
    .map_err(|_| AppError::interno("falha ao gerar hash da senha"))?;

    sqlx::query("INSERT INTO usuario (login, senha_hash, perfil) VALUES (?, ?, 'admin')")
        .bind(login)
        .bind(senha_hash)
        .execute(pool)
        .await?;
    tracing::info!(login, "usuário administrador inicial criado");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bootstrap_valida_limites_antes_de_criar_usuario_e_preserva_contas_existentes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut config = Config::para_testes().unwrap();
        for (login, senha) in [
            ("x".repeat(65), "senha-segura".into()),
            ("admin".into(), "x".repeat(1025)),
            ("ab".into(), "senha-segura".into()),
            ("admin\n".into(), "senha-segura".into()),
            ("admin".into(), "curta".into()),
        ] {
            config.admin_login = Some(login);
            config.admin_password = Some(senha);
            assert!(criar_admin_inicial(&pool, &config).await.is_err());
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usuario")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(total, 0);
        }
        config.admin_login = Some("á".repeat(64));
        config.admin_password = Some("x".repeat(1024));
        criar_admin_inicial(&pool, &config).await.unwrap();
        // Environment credentials are irrelevant after bootstrap, even if invalid.
        config.admin_login = Some("x".repeat(65));
        config.admin_password = Some("curta".into());
        criar_admin_inicial(&pool, &config).await.unwrap();
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usuario")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(total, 1);
    }
}
