use crate::helpers::spawn_app;


#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/health_check", &app.address))
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}