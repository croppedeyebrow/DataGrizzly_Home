//! 개념: 로컬 PostgreSQL 접속과 메타 마이그레이션.
//! 목적: 127.0.0.1:15432에만 붙고, 개발 DB와 격리된 시험 DB를 구분한다.
//! 사용처: `main`의 기동, `cargo test`.

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::ConnectOptions;
use std::str::FromStr;
use std::time::Duration;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 15432;
const DATABASE: &str = "datagrizzly";

pub struct DbConfig {
    url: String,
    user: String,
    password: String,
}

impl DbConfig {
    pub fn from_env() -> Result<Self, String> {
        let url = required("DB_URL")?;
        let user = required("DB_USER")?;
        let password = required("DB_PASSWORD")?;
        validate_url(&url)?;
        if user != DATABASE {
            return Err("DB_USER must be datagrizzly".into());
        }
        Ok(Self { url, user, password })
    }
}

pub async fn connect(config: &DbConfig) -> Result<PgPool, String> {
    let options = connect_options(config)?;
    PgPoolOptions::new()
        .max_connections(4)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|error| redact(&config.password, error))
}

pub async fn migrate(pool: &PgPool) -> Result<(), String> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map(|_| ())
        .map_err(redact_env)
}

#[cfg_attr(not(test), allow(dead_code))]
pub async fn connect_database(database: &str) -> Result<PgPool, String> {
    if database != DATABASE && database != "datagrizzly_m2" {
        return Err("database not allowed".into());
    }
    let user = required("DB_USER")?;
    let password = required("DB_PASSWORD")?;
    if user != DATABASE {
        return Err("DB_USER must be datagrizzly".into());
    }
    let url = format!("postgres://{HOST}:{PORT}/{database}");
    connect(&DbConfig { url, user, password }).await
}

pub async fn ensure_owner(pool: &PgPool) -> Result<(), String> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM owner")
        .fetch_one(pool)
        .await
        .map_err(redact_env)?;
    let password = std::env::var("OWNER_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty());
    let reset = matches!(std::env::var("OWNER_PASSWORD_RESET").as_deref(), Ok("1"));
    if count == 0 {
        let Some(password) = password else {
            return Ok(());
        };
        let hash = crate::auth::hash_password(&password)?;
        sqlx::query("INSERT INTO owner (password_hash) VALUES ($1)")
            .bind(hash)
            .execute(pool)
            .await
            .map_err(redact_env)?;
        return Ok(());
    }
    if reset {
        let Some(password) = password else {
            return Err("OWNER_PASSWORD is required".into());
        };
        let hash = crate::auth::hash_password(&password)?;
        sqlx::query(
            "UPDATE owner SET password_hash = $1
             WHERE id = (
                SELECT id FROM owner WHERE disabled = false ORDER BY created_at ASC LIMIT 1
             )",
        )
        .bind(hash)
        .execute(pool)
        .await
        .map_err(redact_env)?;
    }
    Ok(())
}

fn redact_env(error: impl std::fmt::Display) -> String {
    let password = std::env::var("DB_PASSWORD").unwrap_or_default();
    redact(&password, error)
}

fn required(name: &str) -> Result<String, String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(format!("{name} is required")),
    }
}

fn validate_url(url: &str) -> Result<(), String> {
    if url.contains('@') {
        return Err("DB_URL must not include credentials".into());
    }
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .ok_or("DB_URL must start with postgres://")?;
    let (host_port, database) = rest
        .split_once('/')
        .ok_or("DB_URL needs a database name")?;
    if database != DATABASE || host_port.contains('/') {
        return Err("DB_URL database must be datagrizzly".into());
    }
    let (host, port) = host_port
        .split_once(':')
        .ok_or("DB_URL must be 127.0.0.1:15432")?;
    let port: u16 = port
        .parse()
        .map_err(|_| "DB_URL must be 127.0.0.1:15432")?;
    if host != HOST || port != PORT {
        return Err("DB_URL must be 127.0.0.1:15432".into());
    }
    Ok(())
}

fn connect_options(config: &DbConfig) -> Result<PgConnectOptions, String> {
    let mut options = PgConnectOptions::from_str(&config.url)
        .map_err(|_| "DB_URL must be 127.0.0.1:15432/datagrizzly".to_string())?;
    options = options.username(&config.user).password(&config.password);
    options = options.disable_statement_logging();
    Ok(options)
}

fn redact(password: &str, error: impl std::fmt::Display) -> String {
    let text = error.to_string();
    if !password.is_empty() && text.contains(password) {
        return "database connection failed".into();
    }
    format!("database connection failed: {text}")
}

#[cfg(test)]
mod tests {
    use super::DbConfig;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard;

    impl EnvGuard {
        fn set(url: &str, user: &str, password: &str) -> Self {
            std::env::set_var("DB_URL", url);
            std::env::set_var("DB_USER", user);
            std::env::set_var("DB_PASSWORD", password);
            Self
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("DB_URL");
            std::env::remove_var("DB_USER");
            std::env::remove_var("DB_PASSWORD");
        }
    }

    #[test]
    fn reads_loopback_settings() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _guard = EnvGuard::set(
            "postgres://127.0.0.1:15432/datagrizzly",
            "datagrizzly",
            "secret-value",
        );
        let config = DbConfig::from_env().expect("config");
        assert_eq!(config.url, "postgres://127.0.0.1:15432/datagrizzly");
        assert_eq!(config.user, "datagrizzly");
    }

    #[test]
    fn rejects_native_port_and_credentials() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _guard = EnvGuard::set(
            "postgres://127.0.0.1:5432/datagrizzly",
            "datagrizzly",
            "secret-value",
        );
        assert!(DbConfig::from_env().is_err());

        std::env::set_var(
            "DB_URL",
            "postgres://datagrizzly:secret-value@127.0.0.1:15432/datagrizzly",
        );
        assert!(DbConfig::from_env().is_err());
    }
}
