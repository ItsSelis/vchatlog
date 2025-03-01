use const_format::formatcp as const_format;
use log::{error, info, trace, LevelFilter};
use mysql::{Pool, PooledConn};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, BufReader},
    sync::Arc,
};

static SQL_CONNECTION: Lazy<Arc<Pool>> = Lazy::new(|| {
    simple_logging::log_to_file("vchatlog.log", LevelFilter::Info).unwrap();
    info!(
        "{}",
        const_format!(
            "{name} v{version}",
            name = env!("CARGO_PKG_NAME"),
            version = env!("CARGO_PKG_VERSION")
        )
    );

    let dbconfig = File::open("config/dbconfig.txt").unwrap_or_else(|e| {
        error!("Error while trying to read database configuration: {e}");
        std::process::exit(1);
    });

    let reader = BufReader::new(dbconfig);

    let mut config_map: HashMap<String, String> = Default::default();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                let re = Regex::new(&format!("^([A-Z_]+) (.*?)$")).unwrap_or_else(|e| {
                    error!("Error while setting up regex: {e}");
                    std::process::exit(1);
                });

                match re.captures(&line).ok_or("no match") {
                    Ok(caps) => {
                        let key = caps.get(1).unwrap().as_str();
                        let val = caps.get(2).unwrap().as_str();

                        config_map.insert(key.to_string(), val.to_string());
                    },
                    Err(e) => trace!("Match error: {e}")
                }
            }
            Err(e) => error!("Error while leading line from config/dbconfig.txt: {e}"),
        }
    }

    let db_user = config_map.get("FEEDBACK_LOGIN").unwrap();
    let db_pass = config_map.get("FEEDBACK_PASSWORD").unwrap();
    let db_host = format!(
        "{}:{}",
        config_map.get("ADDRESS").unwrap(),
        config_map.get("PORT").unwrap()
    );
    let db_database = config_map.get("FEEDBACK_DATABASE").unwrap();

    let url = format!("mysql://{db_user}:{db_pass}@{db_host}/{db_database}");

    match Pool::new(url.as_str()) {
        Ok(p) => {
            info!("MariaDB/MySQL connection established.");
            Arc::new(p)
        }
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
        Err(e) => {
            error!("Error while trying to get database connection: {e}");
            panic!("Error while trying to get database connection: {e}")
        }
    }
}
