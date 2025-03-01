use std::time::{SystemTime, UNIX_EPOCH};

use const_format::formatcp as const_format;
use log::{debug, error};
use meowtonin::ByondValue;
use mysql::{params, prelude::Queryable};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{database::get_mariadb_connection, html::parse_html};

/// This function tries to resolves the round_id (as an integer) from the supplied ByondValue.
///
/// NOTE: By default, if the round id's are not set up or some other error occurs, the function will return -1
fn get_round_id(byond_value: ByondValue) -> i32 {
    if byond_value == ByondValue::null() {
        -1
    } else {
        if byond_value.is_number() {
            match byond_value.get_number() {
                Ok(num) => num.trunc() as i32,
                Err(e) => {
                    error!("Failed to get number from ByondValue for round_id: {e}");
                    -1
                }
            }
        } else {
            match byond_value.get_string() {
                Ok(str_value) => match str_value.parse::<i32>() {
                    Ok(num) => num,
                    Err(e) => {
                        error!("Failed to parse string '{str_value}' to i32: {e}");
                        -1
                    }
                },
                Err(e) => {
                    error!("Failed to get string from ByondValue for round_id: {e}");
                    -1
                }
            }
        }
    }
}

#[byond_fn]
pub fn generate_token(ckey: String) -> ByondValue {
    debug!("Writing access token for {ckey}");

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();

    let token: String = (0..32)
        .map(|_| {
            let i = rng.random_range(0..CHARSET.len());
            CHARSET[i] as char
        })
        .collect();

    let mut conn = get_mariadb_connection();

    let token_query = "INSERT INTO chatlogs_ckeys (ckey, token) VALUES (:ckey, :token) ON DUPLICATE KEY UPDATE token = :token";

    if let Err(e) = conn.exec_drop(
        token_query,
        params! {
            "ckey" => ckey.clone(),
            "token" => token.clone()
        },
    ) {
        error!("Error while trying to insert token: {e}");
    };

    debug!("Written access token for {ckey}");

    ByondValue::new_string(token)
}

/// Writes a new changelog to the database for a specific target (ckey).
///
/// NOTE: By default, if round id's are not set up, the round id is -1.
#[byond_fn]
pub fn write_chatlog(
    message_target: String,
    message_html: String,
    message_round_id: ByondValue,
    message_type: String,
) {
    let round_id = get_round_id(message_round_id);

    debug!("Trying to write chatlog for {message_target} ({round_id})");

    // Prepare database for entry
    let mut conn = get_mariadb_connection();

    let parsed_data: crate::html::ParsedData = parse_html(message_html.to_string().as_str());

    // Insert ckey into database, if not existant already
    let ckey_query = "INSERT IGNORE INTO chatlogs_ckeys (ckey) VALUES (:ckey)";
    if let Err(e) = conn.exec_drop(
        ckey_query,
        params! {
            "ckey" => message_target.clone()
        },
    ) {
        error!("Error while trying to insert ckey: {e}");
    };

    // Insert chatlog into database
    let log_query = "INSERT INTO chatlogs_logs (round_id, target, text, text_raw, type, created_at) VALUES (:round_id, :target, :text, :text_raw, :type, :created_at)";
    if let Err(e) = conn.exec_drop(log_query, params! {
        "round_id" => round_id,
        "target" => message_target.to_string(),
        "text" => parsed_data.text,
        "text_raw" => message_html.to_string(),
        "type" => if message_type.is_empty() { None } else { Some(message_type) },
        "created_at" => SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_else(|_| std::time::Duration::new(0, 0)).as_millis()
    }) {
        error!("Error while trying to insert chatlog: {e}")
    };

    debug!("Written chatlog for {message_target} ({round_id}): {message_html}");
}

#[derive(Serialize, Deserialize)]
struct ChatlogEntry {
    round_id: i32,
    text_raw: String,
    msg_type: Option<String>,
    created_at: u128,
}

/// Returns the 10 most recent round ids that have logs recorded.
#[byond_fn]
pub fn get_recent_roundids(ckey: String) -> Vec<ByondValue> {
    let mut conn = get_mariadb_connection();
    let query = "WITH ranked_rounds AS (
            SELECT id, round_id, ROW_NUMBER() OVER (PARTITION BY round_id ORDER BY id DESC) AS rn
            FROM chatlogs_logs 
            WHERE target = :ckey
        )
        SELECT round_id FROM ranked_rounds WHERE rn = 1 ORDER BY id DESC LIMIT 10";

    let results: Vec<i32> = match conn.exec_map(
        query,
        params! {
            "ckey" => ckey.clone()
        },
        |round_id| (round_id),
    ) {
        Ok(results) => results,
        Err(e) => {
            error!("Error while trying to get recent round ids for {ckey}: {e}");
            Vec::new()
        }
    };

    let byond_list: Vec<ByondValue> = results
        .iter()
        .map(|&b| ByondValue::new_string(b.to_string()))
        .collect();

    byond_list
}

#[byond_fn]
pub fn v_chatlog_version() -> &'static str {
    const_format!(
        "{name} v{version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}
