pub mod env_var {
    use std::sync::OnceLock;

    static ENV_VAR: OnceLock<EnvVar> = OnceLock::new();

    #[derive(Debug, Clone)]
    pub struct EnvVar {
        pub database_host: String,
        pub database_port: u16,
        pub database_name: String,
        pub database_user: String,
        pub database_password: String,
        pub database_url: String,
    }

    macro_rules! get_env {
        ($env:literal) => {
            std::env::var($env).expect(concat!("Missing env var ", $env))
        };
    }

    fn load_env() -> EnvVar {
        let database_host = get_env!("DATABASE_HOST");
        let database_name = get_env!("DATABASE_NAME");
        let database_user = get_env!("DATABASE_USER");
        let database_password = get_env!("DATABASE_PASSWORD");
        let database_port: u16 = get_env!("DATABASE_PORT")
            .parse()
            .expect("Invalid DATABASE_PORT");

        let database_url = format!("postgres://{database_user}:{database_password}@{database_host}:{database_port}/{database_name}");

        EnvVar {
            database_host,
            database_name,
            database_password,
            database_port,
            database_user,
            database_url,
        }
    }

    pub fn get() -> &'static EnvVar {
        ENV_VAR.get_or_init(load_env)
    }
}

pub mod connection {
    use std::time::Duration;

    use bb8_postgres::{bb8, PostgresConnectionManager};
    use deadpool_postgres::{ManagerConfig, RecyclingMethod};
    use tokio_postgres_rustls::MakeRustlsConnect;

    use super::env_var;

    fn connection_config() -> tokio_postgres::Config {
        let env = env_var::get();

        let mut cfg = tokio_postgres::Config::new();
        cfg.dbname(&env.database_name);
        cfg.user(&env.database_user);
        cfg.password(env.database_password.clone());
        cfg.port(env.database_port);
        cfg.host(&env.database_host);
        cfg.connect_timeout(Duration::from_millis(5000));
        cfg.application_name("UoW test");
        cfg.ssl_mode(tokio_postgres::config::SslMode::Prefer);
        cfg
    }

    fn tls_config() -> MakeRustlsConnect {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let tls_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        MakeRustlsConnect::new(tls_config)
    }

    fn deadpool_config(cfg: tokio_postgres::Config) -> deadpool_postgres::Config {
        let env = env_var::get();

        let mut config = deadpool_postgres::Config::new();
        config.dbname = cfg.get_dbname().map(ToOwned::to_owned);
        config.user = cfg.get_user().map(ToOwned::to_owned);
        config.password = cfg
            .get_password()
            .map(|pass| String::from_utf8(pass.into()).unwrap());
        config.port = cfg.get_ports().first().copied();
        config.host = Some(env.database_host.clone());
        config.connect_timeout = cfg.get_connect_timeout().cloned();
        config.application_name = Some("UoW test".into());
        config.ssl_mode = Some(deadpool_postgres::SslMode::Prefer);
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        config
    }

    pub fn create_pg_deadpool() -> deadpool_postgres::Pool {
        let config = connection_config();
        let tls = tls_config();

        deadpool_config(config)
            .create_pool(Some(deadpool_postgres::Runtime::Tokio1), tls)
            .unwrap()
    }

    pub type PgBb8pool = bb8::Pool<PostgresConnectionManager<MakeRustlsConnect>>;

    pub async fn create_pg_bb8pool() -> PgBb8pool {
        let config = connection_config();
        let tls = tls_config();
        let manager = PostgresConnectionManager::new(config, tls);

        bb8::Pool::builder()
            .max_size(5)
            .build(manager)
            .await
            .unwrap()
    }

    pub async fn create_sqlx_pool() -> sqlx::PgPool {
        let dburl = env_var::get().database_url.clone();
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(Duration::from_millis(1000))
            .idle_timeout(Duration::from_millis(1000 * 30))
            .max_lifetime(Duration::from_millis(1000 * 10))
            .connect(&dburl)
            .await
            .unwrap()
    }
}

pub mod database {
    use sqlx::{Acquire, Executor};

    pub async fn deadpool_setup_schema(pool: &deadpool_postgres::Pool, command: &str) {
        let mut client = pool.get().await.unwrap();
        let trx = client.transaction().await.unwrap();
        trx.batch_execute(command).await.unwrap();
        trx.commit().await.unwrap();
    }

    pub async fn sqlx_setup_schema(pool: &sqlx::PgPool, command: &str) {
        let mut client = pool.acquire().await.unwrap();
        let mut trx = client.begin().await.unwrap();
        trx.execute(command).await.unwrap();
        trx.commit().await.unwrap();
    }
}
