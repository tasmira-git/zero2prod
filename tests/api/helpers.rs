use once_cell::sync::Lazy;
use reqwest::Url;
use serde_json::Value;
use sqlx::{Connection, PgConnection, PgPool, Executor};
use zero2prod::configuration::{self, DatabaseSettings};
use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry;

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: sqlx::PgPool,
    pub email_server: wiremock::MockServer, 
}
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    } 

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) ->  ConfirmationLinks {
        let body: Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();

            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}

async fn configure_database(config: &DatabaseSettings) -> sqlx::PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"create database "{}""#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate database.");

    connection_pool
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber_name = "test".to_string();
    let default_filter_level = "debug".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = telemetry::get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout
        );
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber = telemetry::get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink
        );
        telemetry::init_subscriber(subscriber);
    };
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = wiremock::MockServer::start().await;

    let configuration = {
        let mut c = configuration::get_configuration().expect("Failed to read configuration.");
        c.database.database_name = uuid::Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await; 

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let application_port = application.port();
    tokio::spawn(application.run_until_stopped());
    
    TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
    }
}