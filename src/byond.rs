use const_format::formatcp as const_format;
use log::error;
use meowtonin::{ByondResult, ByondValue};
use mysql::{params, prelude::Queryable};

use crate::{database::get_mariadb_connection, html::parse_html};

/// Writes a new changelog to the database for a specific target (ckey).
/// 
/// NOTE: By default, if round id's are not set up, the round id is -1.
#[byond_fn]
pub fn write_chatlog(
    message_target: String,
    message_html: String,
    message_round_id: ByondValue
) {
    // Prepare database for entry
    let mut conn = get_mariadb_connection();

    let parsed_data: crate::html::ParsedData = parse_html(message_html.to_string().as_str());

    // Insert ckey into database, if not existant already
    let ckey_query = "INSERT IGNORE INTO ckeys (ckey) VALUES (:ckey)";
    conn.exec_drop(ckey_query, params! {
        "ckey" => message_target.clone()
    }).unwrap();

    // Insert chatlog into database
    let log_query = "INSERT INTO chatlogs (round_id, target, text, text_raw) VALUES (:round_id, :target, :text, :text_raw)";
    conn.exec_drop(log_query, params! {
        "round_id" => if message_round_id == ByondValue::null() { -1 } else { message_round_id.get_number().unwrap() as i32 },
        "target" => message_target.to_string(),
        "text" => parsed_data.text,
        "text_raw" => message_html.to_string()
    }).unwrap();
}

/// Reads the last n chatlogs of a specific ckey, in the order of how they had been written to the database, disregarding the round_id.
/// 
/// By default the length of the chatlogs to fetch is 1000.
#[byond_fn]
pub fn read_chatlog(
    ckey: String,
    length: ByondValue
) -> ByondResult<Vec<ByondValue>> {
    let mut conn = get_mariadb_connection();
    let query = "SELECT TOP (:length) * FROM (SELECT text_raw FROM chatlogs WHERE target = :ckey) AS subquery ORDER BY ID DESC";
    
    let length = length.get_number().unwrap_or_else(|_| 1000.0) as i32;
    let results: Vec<String> = match conn.exec_map(query,
        params! {
            "length" => length,
            "ckey" => ckey
        },
        |text_raw| (text_raw)
    ) {
        Ok(results) => results,
        Err(e) => {
            error!("Error while trying to get last {length} of chatlogs: {e}");
            Vec::new()
        }
    };

    let byond_vec: Vec<ByondValue> = results.into_iter().map(|s| ByondValue::new_string(s)).collect();
    Ok(byond_vec)
}

/// Reads the chatlogs of a specific ckey for a specified round_id.
/// 
/// DANGER: Do not give it the round_id of -1 if you did not have round id's set up for a long time. 
///         Otherwise you might get many results.
#[byond_fn]
pub fn read_chatlog_round(
    ckey: String,
    round_id: ByondValue
) -> ByondResult<Vec<ByondValue>> {
    let mut conn = get_mariadb_connection();
    let query = "SELECT text_raw FROM chatlogs WHERE round_id = :round_id AND target = :ckey ORDER BY ID DESC";

    let parsed_round_id = round_id.get_number().unwrap() as i32;
    let results: Vec<String> = match conn.exec_map(query,
        params! {
            "round_id" => parsed_round_id,
            "ckey" => ckey
        },
        |text_raw| (text_raw)
    ) {
        Ok(results) => results,
        Err(e) => {
            error!("Error while trying to get chatlogs for round {parsed_round_id}: {e}");
            Vec::new()
        }
    };

    let byond_vec: Vec<ByondValue> = results.into_iter().map(|s| ByondValue::new_string(s)).collect();
    Ok(byond_vec)
}

#[byond_fn]
pub fn v_chatlog_version() -> &'static str {
    const_format!(
        "{name} v{version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}