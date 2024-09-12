use axum::{http::StatusCode, Json};
use log::error;
use serde_json::json;
use thiserror::Error;
use tokio_postgres::{Error as PgError, Transaction};

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

// Функция для обработки ошибок транзакции
pub async fn handle_transaction_error<E>(
    err: E,
    transaction: Transaction<'_>,
    message: &str,
) -> (StatusCode, Json<serde_json::Value>)
where
    E: std::fmt::Debug + std::fmt::Display,
{
    // Пытаемся откатить транзакцию
    if let Err(rollback_err) = transaction.rollback().await {
        error!("Failed to rollback transaction: {:?}", rollback_err);
    }

    error!("{}: {}", message, err);

    let error_response = json!({
        "status": "error",
        "message": message,
        "details": err.to_string()
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

// Функция для обработки ошибок получения элементов
pub async fn handle_get_request_error<E>(
    err: E,
    message: &str,
) -> (StatusCode, Json<serde_json::Value>)
where
    E: std::fmt::Debug + std::fmt::Display,
{
    error!("{}: {}", message, err);

    let error_response = json!({
        "error": message,
    });

    (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
}

pub async fn api_fallback() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "status": "Not Found" })),
    )
}
