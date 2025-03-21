use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient};


#[derive(serde::Deserialize)]
pub struct BodyData {
    pub title: String,
    pub content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    pub text: String,
    pub html: String,
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            PublishError::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, 
        }
    }
}
    


pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let subscirbers = get_confirmed_subscriber(&pool).await?;
    for subscriber in subscirbers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.text,
                        &body.content.html
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send email to {}", subscriber.email)
                    })?;
            }
            Err(err) => {
                tracing::warn!("Failed to send email to subscriber: {:?}", err);
            }
            
        }
    }
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Get confirmed subscribers",
    skip(pool)
)]
async fn get_confirmed_subscriber(
    pool: &PgPool
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmd_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(err) => Err(anyhow::anyhow!(err)), 
        }
    })
    .collect();
    Ok(confirmd_subscribers)
}