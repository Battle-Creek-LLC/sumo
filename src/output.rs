use chrono::{DateTime, Utc};
use comfy_table::{Table, ContentArrangement};
use serde_json::Value;
use std::io::{self, Write};

pub fn print_text(results: &[Value], is_aggregate: bool) {
    if results.is_empty() {
        eprintln!("No results found.");
        return;
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.load_preset(comfy_table::presets::NOTHING);

    if is_aggregate {
        // Use all keys from the first record as columns (unwrap "map" if present)
        let first_obj = results[0]
            .get("map")
            .and_then(|m| m.as_object())
            .or_else(|| results[0].as_object());
        if let Some(obj) = first_obj {
            let keys: Vec<String> = obj.keys().cloned().collect();
            let headers: Vec<String> = keys.iter().map(|k| k.to_uppercase()).collect();
            table.set_header(&headers);

            for record in results {
                let rec_obj = record
                    .get("map")
                    .and_then(|m| m.as_object())
                    .or_else(|| record.as_object());
                if let Some(obj) = rec_obj {
                    let row: Vec<String> = keys.iter().map(|k| {
                        obj.get(k).map(value_to_string).unwrap_or_default()
                    }).collect();
                    table.add_row(row);
                }
            }
        }
    } else {
        table.set_header(vec!["TIME", "SOURCE", "MESSAGE"]);

        for msg in results {
            let time_raw = msg
                .get("map")
                .and_then(|m| m.get("_messagetime"))
                .or_else(|| msg.get("_messagetime"))
                .map(value_to_string)
                .unwrap_or_default();
            let time = format_epoch_ms(&time_raw);
            let source = msg
                .get("map")
                .and_then(|m| m.get("_sourcecategory"))
                .or_else(|| msg.get("_sourcecategory"))
                .or_else(|| msg.get("map").and_then(|m| m.get("_sourceCategory")))
                .or_else(|| msg.get("_sourceCategory"))
                .map(value_to_string)
                .unwrap_or_default();
            let raw = msg
                .get("map")
                .and_then(|m| m.get("_raw"))
                .or_else(|| msg.get("_raw"))
                .map(value_to_string)
                .unwrap_or_default();

            // Truncate message for display
            let display_raw = if raw.len() > 120 {
                format!("{}...", &raw[..120])
            } else {
                raw
            };

            table.add_row(vec![time, source, display_raw]);
        }
    }

    println!("{table}");
}

pub fn print_json(results: &[Value]) {
    let json = serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string());
    println!("{json}");
}

pub fn print_csv(results: &[Value]) -> Result<(), String> {
    if results.is_empty() {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Collect all unique keys for headers
    let mut headers: Vec<String> = Vec::new();
    for result in results {
        let obj = result
            .get("map")
            .and_then(|m| m.as_object())
            .or_else(|| result.as_object());
        if let Some(obj) = obj {
            for key in obj.keys() {
                if !headers.contains(key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&headers).map_err(|e| format!("CSV error: {e}"))?;

    for result in results {
        let obj = result
            .get("map")
            .and_then(|m| m.as_object())
            .or_else(|| result.as_object());
        let row: Vec<String> = headers
            .iter()
            .map(|h| {
                obj.and_then(|o| o.get(h))
                    .map(value_to_string)
                    .unwrap_or_default()
            })
            .collect();
        wtr.write_record(&row).map_err(|e| format!("CSV error: {e}"))?;
    }

    let data = wtr.into_inner().map_err(|e| format!("CSV error: {e}"))?;
    handle.write_all(&data).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

pub fn print_raw(results: &[Value]) {
    for msg in results {
        let raw = msg
            .get("map")
            .and_then(|m| m.get("_raw"))
            .or_else(|| msg.get("_raw"))
            .map(value_to_string)
            .unwrap_or_default();
        println!("{raw}");
    }
}

pub fn print_fields(results: &[Value], fields: &[String]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.load_preset(comfy_table::presets::NOTHING);

    let headers: Vec<String> = fields.iter().map(|f| f.to_uppercase()).collect();
    table.set_header(&headers);

    for result in results {
        let obj = result
            .get("map")
            .and_then(|m| m.as_object())
            .or_else(|| result.as_object());
        let row: Vec<String> = fields
            .iter()
            .map(|f| {
                obj.and_then(|o| o.get(f))
                    .map(value_to_string)
                    .unwrap_or_default()
            })
            .collect();
        table.add_row(row);
    }

    println!("{table}");
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn format_epoch_ms(s: &str) -> String {
    if let Ok(ms) = s.parse::<i64>() {
        if let Some(dt) = DateTime::<Utc>::from_timestamp(ms / 1000, ((ms % 1000) * 1_000_000) as u32) {
            return dt.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        }
    }
    s.to_string()
}
