use std::{fmt::Display, ops::DerefMut};

use anyhow::Context;
use rand::{distr::Alphanumeric, Rng};
use serde::Deserialize;
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use sqlx::{PgPool, Transaction};
use uuid::Uuid;
use chrono::Utc;
use crate::{domain::NewSubscriber, email_client::EmailClient, startup::ApplicationBaseUrl};

#[derive(Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber.",
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscriptions(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>
) -> Result<HttpResponse, SubscirbeError> {
    let mut transaction = db_pool.begin().await
        .context("Failed to acquire a Postgres connection from the pool.")?;

    let new_subscriber = form.0.try_into()
        .map_err(SubscirbeError::ValidationError)?;

    let subscriber_id = insert_subscriber(&new_subscriber, &mut transaction).await
        .context("Failed to insert new subscriber in the database.")?;
    let subscriber_token = generate_subscription_token();
    
    store_token(&mut transaction, subscriber_id, &subscriber_token).await
        .context("Failed to store the confirmation token for a new subscriber.")?;

    transaction.commit().await.context("Failed to commit SQL transaction to store a new subscriber")?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscriber_token
    ).await
    .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

fn generate_subscription_token() -> String {
    let mut rng = rand::rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}



#[tracing::instrument(
    name = "Storing subscription token in the database.",
    skip(transaction, subscription_token),
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, sqlx::Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"
        insert into subscription_tokens (subscription_token, subscription_id)
        values ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    )
    .execute(transaction.deref_mut())
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Sending confirmation email to new subscriber.",
    skip(email_client, new_subscriber, base_url, subscription_token),
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?subscription_token={}", base_url, subscription_token);

    let plain_body = format!(
            "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
            confirmation_link
    );

    let html_body = format!(
            "Welcome to our newsletter!<br />\
            Click <a href=\"{}\">here</a> to confirm your subscription.",
            confirmation_link
    );
        
    email_client.send_email(
        &new_subscriber.email,
        "Welcome!",
        &html_body,
        &plain_body
    ).await

}


#[tracing::instrument(
    name = "Saving new subscriber details in the database.",
    skip(new_subscriber, transaction),
)]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, sqlx::Postgres> 
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        insert into subscriptions (id, email, name, subscribed_at, status)
        values ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now() 
    ) 
    .execute(transaction.deref_mut())
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

#[derive(Debug, thiserror::Error)]
pub enum SubscirbeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscirbeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::ValidationError(_) =>StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR, 
        }
    }
}

#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);
impl Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A database error occurred while storing the subscription token.")
    } 
}
impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
    
}
impl ResponseError for StoreTokenError { }
