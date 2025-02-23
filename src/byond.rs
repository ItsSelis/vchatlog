use std::fs;

use const_format::formatcp as const_format;
use log::{debug, error, info};
use meowtonin::ByondValue;
use mysql::{params, prelude::Queryable};

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
                Ok(str_value) => {
                    match str_value.parse::<i32>() {
                        Ok(num) => num,
                        Err(e) => {
                            error!("Failed to parse string '{str_value}' to i32: {e}");
                            -1
                        }
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

/// Writes a new changelog to the database for a specific target (ckey).
/// 
/// NOTE: By default, if round id's are not set up, the round id is -1.
#[byond_fn]
pub fn write_chatlog(
    message_target: String,
    message_html: String,
    message_round_id: ByondValue
) {
    let round_id = get_round_id(message_round_id);

    debug!("Trying to write chatlog for {message_target} ({round_id})");

    // Prepare database for entry
    let mut conn = get_mariadb_connection();

    let parsed_data: crate::html::ParsedData = parse_html(message_html.to_string().as_str());

    // Insert ckey into database, if not existant already
    let ckey_query = "INSERT IGNORE INTO ckeys (ckey) VALUES (:ckey)";
    if let Err(e) = conn.exec_drop(ckey_query, params! {
        "ckey" => message_target.clone()
    }) {
        error!("Error while trying to insert ckey: {e}");
    };

    // Insert chatlog into database
    let log_query = "INSERT INTO chatlogs (round_id, target, text, text_raw) VALUES (:round_id, :target, :text, :text_raw)";
    if let Err(e) = conn.exec_drop(log_query, params! {
        "round_id" => round_id,
        "target" => message_target.to_string(),
        "text" => parsed_data.text,
        "text_raw" => message_html.to_string()
    }) {
        error!("Error while trying to insert chatlog: {e}")
    };

    debug!("Written chatlog for {message_target} ({round_id}): {message_html}");
}

/// Reads the last n chatlogs of a specific ckey, in the order of how they had been written to the database, disregarding the round_id.
/// 
/// By default the length of the chatlogs to fetch is 1000.
#[byond_fn]
pub fn read_chatlog(
    ckey: String,
    length: ByondValue,
    rendered: bool
) {
    let mut conn = get_mariadb_connection();
    let query = "SELECT TOP (:length) * FROM (SELECT text_raw FROM chatlogs WHERE target = :ckey) AS subquery ORDER BY ID ASC";
    
    let length = length.get_number().unwrap_or_else(|_| 1000.0) as i32;
    let results: Vec<String> = match conn.exec_map(query,
        params! {
            "length" => length,
            "ckey" => ckey.clone()
        },
        |text_raw| (text_raw)
    ) {
        Ok(results) => results,
        Err(e) => {
            error!("Error while trying to get last {length} of chatlogs: {e}");
            Vec::new()
        }
    };

    info!("Exporting last {length} messages of {ckey}");
    fs::create_dir_all("tmp/chatlogs").unwrap_or_else(|e| error!("Error while trying to create temporary chatlogs directory: {e}"));
    if rendered {
        fs::write(
            format!("tmp/chatlogs/{ckey}.html"), 
            format!(
                "<!DOCTYPE html><html><head><title>SS13 Chat Log</title></head><body><div class=\"Chat\">{}</div></body></html>",
                results.iter()
                    .map(|msg| format!("<div class=\"ChatMessage\">{}</div>", msg))
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        ).unwrap_or_else(|e| error!("Error while trying to write chatlogs to file (last {length}): {e}") );
    } else {
        fs::write(
            format!("tmp/chatlogs/{ckey}"), 
            format!(
                "{}",
                results.iter()
                    .map(|msg| format!("<div class=\"ChatMessage\">{}</div>", msg))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        ).unwrap_or_else(|e| error!("Error while trying to write chatlogs to file (last {length}): {e}") );
    }
}

/// Reads the chatlogs of a specific ckey for a specified round_id.
/// 
/// DANGER: Do not give it the round_id of -1 if you did not have round id's set up for a long time. 
///         Otherwise you might get many results.
#[byond_fn]
pub fn read_chatlog_round(
    ckey: String,
    round_id: ByondValue,
    rendered: bool
) {
    let mut conn = get_mariadb_connection();
    let query = "SELECT text_raw FROM chatlogs WHERE round_id = :round_id AND target = :ckey ORDER BY ID ASC";

    let parsed_round_id = get_round_id(round_id);
    let results: Vec<String> = match conn.exec_map(query,
        params! {
            "round_id" => parsed_round_id,
            "ckey" => ckey.clone()
        },
        |text_raw| (text_raw)
    ) {
        Ok(results) => results,
        Err(e) => {
            error!("Error while trying to get chatlogs for round {parsed_round_id}: {e}");
            Vec::new()
        }
    };

    info!("Exporting chatlog for {ckey} for round {parsed_round_id}");
    fs::create_dir_all("tmp/chatlogs").unwrap_or_else(|e| error!("Error while trying to create temporary chatlogs directory: {e}"));
    if rendered {
        fs::write(
            format!("tmp/chatlogs/{ckey}-{parsed_round_id}.html"), 
            format!(
                "<!DOCTYPE html><html><head><title>SS13 Chat Log - Round {parsed_round_id}</title></head><body><div class=\"Chat\">{}</div></body></html>",
                results.iter()
                    .map(|msg| format!("<div class=\"ChatMessage\">{}</div>", msg))
                    .collect::<Vec<String>>()
                    .join("\n"),
            )
        ).unwrap_or_else(|e| error!("Error while trying to write chatlogs to file for round {parsed_round_id}: {e}") );
    } else {
        fs::write(
            format!("tmp/chatlogs/{ckey}-{parsed_round_id}"), 
            format!(
                "{}",
                results.iter()
                    .map(|msg| format!("<div class=\"ChatMessage\">{}</div>", msg))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        ).unwrap_or_else(|e| error!("Error while trying to write chatlogs to file for round {parsed_round_id}: {e}") );
    }
}

#[byond_fn]
pub fn v_chatlog_version() -> &'static str {
    const_format!(
        "{name} v{version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}