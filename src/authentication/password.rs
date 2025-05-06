use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, Params, PasswordHash, PasswordVerifier, PasswordHasher};
use sqlx::PgPool;
use anyhow::Context;

use crate::telemetry::spawn_blocking_with_tracing;

pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)] 
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(
    name = "Validate credentials",
    skip(credentials, pool)
)]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash =
        "$argon2id$v=19$m=19456,t=2,p=1\
        $9Dlp+zTp4VX2ZD8BSX9L5A\
        $/gENNkTsIGr98GC+vlmLj6x5FGwXd/8zUKAnuuRiPj4"
        .to_string();

    let row = get_store_credentials(&credentials.username, pool)
        .await?;

    if let Some((store_user_id, store_password_hash)) = row {
        user_id = Some(store_user_id);
        expected_password_hash = store_password_hash;
    };

    spawn_blocking_with_tracing(move || verify_password_hash(expected_password_hash, credentials.password))
        .await
        .context("Failed to spawn blocking task")??;

    // tokio::task::spawn_blocking(move || {
    //     let current_span = tracing::Span::current();
    //     current_span.in_scope(|| verify_password_hash(expected_password_hash, credentials.password))
    // })
    // .await
    // .context("Failed to spawn blocking task")??;
        
    user_id.ok_or_else(|| {
        AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username."))
    })
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password)
)]
fn verify_password_hash(
    expected_password_hash: String,
    password: String,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default().verify_password(password.as_bytes(), &expected_password_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)

}

#[tracing::instrument(
    name = "Get store credentials",
    skip(username, pool)
)]
async fn get_store_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, String)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?
    .map(|row| (row.user_id, row.password_hash));

    Ok(row)
}

pub async fn change_password(
    user_id: uuid::Uuid,
    password: String,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash,
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;
    Ok(())
}

fn compute_password_hash(
    password: String,
) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap()
    )
    .hash_password(password.as_bytes(), &salt)?
    .to_string();
    Ok(password_hash)
}