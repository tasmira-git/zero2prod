use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn login_form(
    flash_message: IncomingFlashMessages,
) -> HttpResponse {
    let mut error_html = String::new();
    for m in flash_message.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login</title>
</head>
<body>
    {error_html}
    <form action="/login" method="POST">
        <label>Username
            <input
                type="text"
                placeholder="Enter your username"
                name="username"
            >
        </label>

        <label>Password
            <input
                type="password"
                placeholder="Enter your password"
                name="password"
            >
        </label>

        <button type="submit">Login</button>
    </form>
    
</body>
</html>"#,
        ))
}