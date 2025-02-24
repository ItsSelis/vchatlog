use const_format::formatcp as const_format;
use log::{error, info, warn, LevelFilter};
use mysql::{Pool, PooledConn};
use once_cell::sync::Lazy;
use std::{fs, sync::Arc};

static SQL_CONNECTION: Lazy<Arc<Pool>> = Lazy::new(|| {
    simple_logging::log_to_file("vchatlog.log", LevelFilter::Info).unwrap();
    info!("{}", const_format!(
        "{name} v{version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    ));

    // Clean up old stuff
    fs::remove_dir_all("data/chatlogs").unwrap_or_else(|e| warn!("Error while trying to create temporary chatlogs directory: {e}"));

    let db_user = std::env::var("DB_USER").expect("environment variable DB_USER");
    let db_pass = std::env::var("DB_PASS").expect("environment variable DB_PASS");
    let db_host = std::env::var("DB_HOST").expect("environment variable DB_HOST");
    let db_database = std::env::var("DB_DATABASE").expect("environment variable DB_DATABASE");

    let url = format!("mysql://{db_user}:{db_pass}@{db_host}/{db_database}");

    match Pool::new(url.as_str()) {
        Ok(p) => {
            info!("MariaDB/MySQL connection established.");
            Arc::new(p)
        },
        Err(e) => {
            error!("Failed to connect to MariaDB/MySQL: {}", e);
            std::process::exit(1);
        }
    }
});

/// Returns the current database connection the library was initialized with.
pub fn get_mariadb_connection() -> PooledConn {
    match Arc::clone(&SQL_CONNECTION).get_conn() {
        Ok(conn) => conn,
        Err(e ) => {
            error!("Error while trying to get database connection: {e}");
            panic!("Error while trying to get database connection: {e}")
        }
    }
}