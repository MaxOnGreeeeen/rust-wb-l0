use std::{env, fmt, fs, path::Path, str::FromStr, sync::Arc};

use log::{error, info};
use tokio_postgres::GenericClient;

use crate::{errors::AppError, AppState};

// Типы миграций
#[derive(Debug, Clone)]
pub enum Migration {
    None,
    Up,
    Down,
}

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Migration::None => write!(f, "none"),
            Migration::Up => write!(f, "up"),
            Migration::Down => write!(f, "down"),
        }
    }
}

impl FromStr for Migration {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "up" => Ok(Migration::Up),
            "down" => Ok(Migration::Down),
            "none" => Ok(Migration::None),
            &_ => unimplemented!(),
        }
    }
}

// Применяет миграции в зависимости от переданных аргументов
pub async fn migrate(app_state: Arc<AppState>, migration: Migration) -> Result<(), AppError> {
    let migration_name = match migration {
        Migration::Up => "init_migration.sql",
        Migration::Down => "down_migration.sql",
        Migration::None => "",
    };

    let resolved_migration_script_string = match load_migration_script_as_string(migration_name) {
        Ok(migration) => migration,
        Err(err) => {
            return Err(err)?;
        }
    };

    let client_db = app_state.db.lock().await;
    for migration_script in resolved_migration_script_string
        .split(";")
        .collect::<Vec<&str>>()
    {
        client_db.client().execute(migration_script, &[]).await?;
    }

    info!("Succefully migrated!");

    Ok(())
}

fn load_migration_script_as_string(migration_name: &str) -> Result<String, std::io::Error> {
    let migration_script_path_string = format!("./src/migrations/{}", &migration_name);

    let migration_script_path = match Path::new(&migration_script_path_string).canonicalize() {
        Ok(path) => path,
        Err(err) => {
            error!("Incorrect migration path!");
            return Err(err)?;
        }
    };

    let migration_script_path_abs_path = env::current_dir().unwrap().join(migration_script_path);

    let resolved_migration_script = fs::read_to_string(migration_script_path_abs_path)?;

    Ok(resolved_migration_script)
}
