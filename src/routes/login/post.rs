use actix_web::{error::InternalError, http::header::LOCATION, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use sqlx::PgPool;
use crate::session_state::TypedSession;
use crate::authentication::{validate_credentials, AuthError, Credentials};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: String,
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(
    skip(form, pool, session),
    fields(
        username = tracing::field::Empty,
        user_id = tracing::field::Empty
    )
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.renew();
            session.insert_user_id(user_id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish()
            )
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()), 
            };

            FlashMessage::error(e.to_string()).send();

            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION,"/login"))
                // .cookie(Cookie::new("_flash", e.to_string()))
                .finish();

            Err(InternalError::from_response(e, response))
        }
    }
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();

    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
    InternalError::from_response(e, response)
}
