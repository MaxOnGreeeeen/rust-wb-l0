use std::{sync::Arc, time::Duration};

use axum::{
    routing::{get, post},
    Router,
};
use cache::Cache;
use errors::{api_fallback, AppError};
use migrate::Migration;
use schema::GetOrderDTO;
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls};

mod cache;
mod errors;
mod fill_test_data;
mod migrate;
mod routes;
mod schema;
mod utils;
use clap::Parser;

use crate::routes::{create_order_handler, get_order_handler};
use log::{error, info, warn};

/// Orders service
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Number of orders to create
    #[arg(long, default_value_t = 1)]
    count: u64,

    /// Delay between requests
    #[arg(short, long, default_value_t = 1000)]
    delay: u64,

    /// Number of tokio threads
    #[arg(long, default_value_t = 8)]
    threads: u8,

    /// Target app port
    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    /// Migraion script
    #[arg(short, long, value_enum)]
    migration: Option<Migration>,

    /// Run test data script
    #[clap(long, action)]
    test_run: bool,
}

pub struct AppState {
    db: Arc<Mutex<Client>>,
    cache: Arc<Mutex<Cache<GetOrderDTO>>>,
}

// Создание роутера
fn create_router(app_state: Arc<AppState>) -> Router {
    return Router::new()
        .route("/api/orders/:id", get(get_order_handler))
        .route("/api/orders", post(create_order_handler))
        .fallback(api_fallback)
        .with_state(app_state);
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    utils::init_logger();

    let args_arc = Arc::new(Args::parse());
    let (client, connection) =
        tokio_postgres::connect(&utils::build_connection_string(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Database connection error: {e}");
        }
    });

    let cache: Cache<GetOrderDTO> = cache::Cache::new();
    let app_state = Arc::new(AppState {
        db: Arc::new(Mutex::new(client)),
        cache: Arc::new(Mutex::new(cache)),
    });

    match args_arc.migration.clone().unwrap_or(Migration::None) {
        Migration::None => {}
        migration => {
            let app_state_clone: Arc<AppState> = app_state.clone();
            tokio::spawn(async move {
                if let Err(e) = migrate::migrate(app_state_clone, migration.clone()).await {
                    error!("Migration error: {e}");
                }
            });
        }
    }

    let router = create_router(app_state.clone());
    let port_connection = args_arc.port;
    let socket_addr = format!("0.0.0.0:{}", port_connection);

    let listener = tokio::net::TcpListener::bind(&socket_addr).await.unwrap();

    info!("🚀 Server started successfully on port: {port_connection}");

    if args_arc.test_run {
        let args_arc_clone = args_arc.clone();
        tokio::spawn(async move {
            warn!("Start testing");

            let _ = fill_test_data::fill_test_data(args_arc_clone).await;

            warn!("End testing");
        });
    }
    {
        let cache_clone = app_state.cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(900));
            // Подчищаем кеш каждые 15 минут
            loop {
                interval.tick().await;
                cache_clone.lock().await.cleanup_expired();
            }
        });
    }

    axum::serve(listener, router).await.unwrap();

    Ok(())
}
