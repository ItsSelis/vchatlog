use std::time::{SystemTime, UNIX_EPOCH};

use const_format::formatcp as const_format;
use log::{debug, error};
use meowtonin::{byond_fn, ByondValue, ByondValueType};
use mysql::{params, prelude::Queryable};
use rand::distr::{Alphanumeric, SampleString};

use crate::{database::get_mariadb_connection, html::parse_html};

/// This function tries to resolve the round_id (as an integer) from the supplied ByondValue.
///
/// NOTE: By default, if the round ids are not set up or some other error occurs, the function will return -1
fn get_round_id(value: ByondValue) -> i32 {
    fn get_round_id_inner(value: ByondValue) -> Result<i32, String> {
        let value_type = value.get_type();
        match value_type {
            ByondValueType::Null => Ok(-1),
            ByondValueType::Number => value
                .get_number()
                .map_err(|err| format!("Failed to get number from ByondValue for round_id: {err}"))
                .map(|num| num.trunc() as i32),
            ByondValueType::String => value
                .get_string()
                .map_err(|err| format!("Failed to get string from ByondValue for round_id: {err}"))
                .and_then(|string| {
                    string
                        .parse::<i32>()
                        .map_err(|err| format!("Failed to parse string '{string}' to i32: {err}"))
                }),
            _ => Err(format!(
                "round_id ByondValue was not a valid type ({value_type}, {value})",
                value_type = value_type.name()
            )),
        }
    }

    get_round_id_inner(value).unwrap_or_else(|err| {
        error!("{err}");
        -1
    })
}

#[byond_fn]
pub fn generate_token(ckey: String, message_round_id: ByondValue) -> ByondValue {
    let round_id = get_round_id(message_round_id);

    debug!("Writing access token for {ckey}");
    let token = Alphanumeric.sample_string(&mut rand::rng(), 32);

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

    // Call the procedure that inserts the round id into the rounds table, and deletes the oldest round if there are more than 10 rounds for this ckey.
    let round_query = "CALL chatlogs_rounds_insert(:round_id, :ckey)";
    if let Err(e) = conn.exec_drop(
        round_query,
        params! {
            "round_id" => round_id,
            "ckey" => ckey
        },
    ) {
        error!("Error while trying to insert round id to chatlogs_rounds: {e}");
    };

    ByondValue::new_string(token)
}

/// Writes a new changelog to the database for a specific target (ckey).
///
/// NOTE: By default, if round ids are not set up, the round id is -1.
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

    let parsed_data = parse_html(message_html.as_str());

    // Insert ckey into database, if not existent already
    let ckey_query = "INSERT IGNORE INTO chatlogs_ckeys (ckey) VALUES (:ckey)";
    if let Err(e) = conn.exec_drop(
        ckey_query,
        params! {
            "ckey" => message_target.clone()
        },
    ) {
        error!("Error while trying to insert ckey to chatlogs_ckeys: {e}");
    };

    // Call the procedure that inserts the round id into the rounds table, and deletes the oldest round if there are more than 10 rounds for this ckey.
    // Just to make sure it really is inserted.
    let round_query = "CALL chatlogs_rounds_insert(:round_id, :ckey)";
    if let Err(e) = conn.exec_drop(
        round_query,
        params! {
            "round_id" => round_id,
            "ckey" => message_target.clone()
        },
    ) {
        error!("Error while trying to insert round id to chatlogs_rounds: {e}");
    };

    // Insert chatlog into database
    let log_query = "INSERT INTO chatlogs_logs (round_id, target, text, text_raw, type, created_at) VALUES (:round_id, :target, :text, :text_raw, :type, :created_at)";
    if let Err(e) = conn.exec_drop(log_query, params! {
        "round_id" => round_id,
        "target" => message_target.clone(),
        "text" => parsed_data.text,
        "text_raw" => message_html.clone(),
        "type" => if message_type.is_empty() { None } else { Some(message_type) },
        "created_at" => SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_else(|_| std::time::Duration::new(0, 0)).as_millis()
    }) {
        error!("Error while trying to insert chatlog: {e}")
    };

    debug!("Written chatlog for {message_target} ({round_id}): {message_html}");
}

/// Returns the 10 most recent round ids that have logs recorded.
#[byond_fn]
pub fn get_recent_roundids(ckey: String) -> Vec<ByondValue> {
    let mut conn = get_mariadb_connection();
    let query = "SELECT round_id FROM chatlogs_rounds WHERE ckey = :ckey ORDER BY round_id DESC LIMIT 10";

    let results: Vec<i32> = conn.exec_map(
        query,
        params! {
            "ckey" => ckey.clone()
        },
        |round_id| round_id,
    ).unwrap_or_else(|e| {
        error!("Error while trying to get recent round ids for {ckey}: {e}");
        Vec::new()
    });

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
