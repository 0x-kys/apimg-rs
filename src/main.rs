use axum::{
    extract::Multipart,
    routing::{get, post},
    Router,
    response::{Json, Html},
    http::StatusCode,
};
use serde::Serialize;
use std::{net::SocketAddr, path::Path};
use tower_http::{
    trace::TraceLayer,
    services::ServeDir,
    cors::CorsLayer,
};
use tracing_subscriber;
use uuid::Uuid;

#[derive(Serialize)]
struct UploadResponse {
    message: String,
    filenames: Vec<String>,
}

const UPLOAD_DIR: &str = "uploads";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tokio::fs::create_dir_all(UPLOAD_DIR).await.expect("Failed to create upload directory");

    let html_content = include_str!("../static/index.html");

    let app = Router::new()
        .route("/", get(move || async move { Html(html_content) }))
        .route("/upload", post(upload_images))
        .nest_service("/uploads", ServeDir::new(UPLOAD_DIR))
        .layer(CorsLayer::permissive()) // Enable CORS for local development
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server starting on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn upload_images(mut multipart: Multipart) -> Result<Json<UploadResponse>, StatusCode> {
    let mut saved_filenames = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let original_filename = match field.file_name() {
            Some(filename) => filename.to_string(),
            None => continue,
        };

        let extension = Path::new(&original_filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");

        let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let file_path = Path::new(UPLOAD_DIR).join(&unique_filename);

        let data = match field.bytes().await {
            Ok(data) => data,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        if let Err(_) = tokio::fs::write(&file_path, data).await {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        saved_filenames.push(unique_filename);
    }

    if saved_filenames.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(Json(UploadResponse {
        message: "Files uploaded successfully".to_string(),
        filenames: saved_filenames,
    }))
}