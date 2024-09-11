use dotenv::dotenv;
use env_logger::Env;

pub fn build_connection_string() -> String {
    dotenv().ok();

    let pg_user = std::env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
    let pg_port = std::env::var("POSTGRES_PORT").expect("POSTGRES_PORT must be set");
    let pg_host = std::env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be set");
    let pg_password = std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
    let pg_db = std::env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");

    format!("user={pg_user} password={pg_password} dbname={pg_db} host={pg_host} port={pg_port}")
}

pub fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
}
