use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use crate::routes::{subscriptions, health_check};
use crate::send_email::EmailClient;
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool, email_client: EmailClient) -> std::io::Result<Server> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscriptions))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}