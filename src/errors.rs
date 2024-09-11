use axum::{http::StatusCode, Json};
use serde_json::json;
use thiserror::Error;
use tokio_postgres::Error as PgError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Postgres error: {0}")]
    PostgresError(#[from] PgError),

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("UID parse error: {0}")]
    UIDError(#[from] uuid::Error),
}

pub fn handle_db_error(err: PgError) -> (StatusCode, Json<serde_json::Value>) {
    let error_response = json!({
        "status": "error",
        "message": format!("Database error: {:?}", err),
    });
    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

pub fn handle_create_order_error(err: AppError) -> (StatusCode, Json<serde_json::Value>) {
    let error_response = json!({
        "status": "error",
        "message": format!("Create order error: {:?}", err),
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

pub fn handle_create_delivery_error(err: PgError) -> (StatusCode, Json<serde_json::Value>) {
    let error_response = json!({
        "status": "error",
        "message": format!("Create order error: {:?}", err),
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

pub fn handle_create_payment_error(err: PgError) -> (StatusCode, Json<serde_json::Value>) {
    let error_response = json!({
        "status": "error",
        "message": format!("Create payment error: {:?}", err),
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

pub fn handle_create_order_items(err: PgError) -> (StatusCode, Json<serde_json::Value>) {
    let error_response = json!({
        "status": "error",
        "message": format!("Create items error: {:?}", err),
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

pub async fn api_fallback() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "status": "Not Found" })),
    )
}
