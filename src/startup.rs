use actix_session::SessionMiddleware;
use actix_web::{cookie::Key, dev::Server, middleware::from_fn, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::TcpListener;
use crate::{
    authentication::reject_anonymous_users,
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{admin_dashboard, change_password, change_password_form, confirm, health_check, home, login, login_form, logout, publish_newsletter, publish_newsletter_form, subscriptions}
};
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let sender_email = configuration.email_client.sender().expect("Invalid email");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.auth_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host,
            configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let connection_pool = get_connection_pool(&configuration.database);
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_url,
        ).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}


pub fn get_connection_pool(database_config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(database_config.with_db())
}

pub struct ApplicationBaseUrl(pub String);
pub struct HmacSecret(pub String);

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: String,
    redis_url: String,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let secret_key = Key::from(hmac_secret.as_bytes());
    let message_store = CookieMessageStore::builder(
        secret_key.clone() 
    ).build();
    let message_framwork = FlashMessagesFramework::builder(message_store).build();
    let redis_store = actix_session::storage::RedisSessionStore::new(redis_url).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(message_framwork.clone())
            .wrap(SessionMiddleware::new(redis_store.clone(), secret_key.clone()))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .service(
                web::scope("/admin")
                        .wrap(from_fn(reject_anonymous_users))
                        .route("/dashboard", web::get().to(admin_dashboard))
                        .route("/newsletter", web::get().to(publish_newsletter_form))
                        .route("/newsletter", web::post().to(publish_newsletter))
                        .route("/password", web::get().to(change_password_form))
                        .route("/password", web::post().to(change_password))
                        .route("/logout", web::post().to(logout))
            )
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscriptions))
            .route("subscriptions/confirm", web::get().to(confirm))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}