use chrono::{DateTime, Utc};
use comfy_table::{Table, ContentArrangement};
use serde_json::{Map, Value};
use std::io::{self, Write};

/// Extract the inner "map" object from a result, or use the result directly.
fn unwrap_map(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    v.get("map")
        .and_then(|m| m.as_object())
        .or_else(|| v.as_object())
}

/// Filter results to only include the specified fields.
/// Returns new Value objects with only the requested keys.
pub fn filter_fields(results: &[Value], fields: &[String]) -> Vec<Value> {
    results
        .iter()
        .map(|result| {
            let obj = unwrap_map(result);
            let mut filtered = Map::new();
            if let Some(obj) = obj {
                for field in fields {
                    if let Some(val) = obj.get(field) {
                        filtered.insert(field.clone(), val.clone());
                    }
                }
            }
            Value::Object(filtered)
        })
        .collect()
}

pub fn print_text(results: &[Value], is_aggregate: bool) {
    if results.is_empty() {
        eprintln!("No results found.");
        return;
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.load_preset(comfy_table::presets::NOTHING);

    if is_aggregate {
        let first_obj = unwrap_map(&results[0]);
        if let Some(obj) = first_obj {
            let keys: Vec<String> = obj.keys().cloned().collect();
            let headers: Vec<String> = keys.iter().map(|k| k.to_uppercase()).collect();
            table.set_header(&headers);

            for record in results {
                let rec_obj = unwrap_map(record);
                if let Some(obj) = rec_obj {
                    let row: Vec<String> = keys
                        .iter()
                        .map(|k| obj.get(k).map(value_to_string).unwrap_or_default())
                        .collect();
                    table.add_row(row);
                }
            }
        }
    } else {
        table.set_header(vec!["TIME", "SOURCE", "MESSAGE"]);

        for msg in results {
            let obj = unwrap_map(msg);
            let time_raw = obj
                .and_then(|o| o.get("_messagetime"))
                .map(value_to_string)
                .unwrap_or_default();
            let time = format_epoch_ms(&time_raw);
            let source = obj
                .and_then(|o| o.get("_sourcecategory").or_else(|| o.get("_sourceCategory")))
                .map(value_to_string)
                .unwrap_or_default();
            let raw = obj
                .and_then(|o| o.get("_raw"))
                .map(value_to_string)
                .unwrap_or_default();

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

/// Print JSON output. When fields are pre-filtered, objects are flat (no "map" wrapper).
pub fn print_json(results: &[Value], fields_filtered: bool) {
    let output: Vec<&Value> = if fields_filtered {
        results.iter().collect()
    } else {
        results.iter().collect()
    };

    // Unwrap "map" wrapper for cleaner JSON when not pre-filtered
    if !fields_filtered {
        let unwrapped: Vec<Value> = results
            .iter()
            .map(|r| {
                if let Some(map) = r.get("map") {
                    map.clone()
                } else {
                    r.clone()
                }
            })
            .collect();
        let json =
            serde_json::to_string_pretty(&unwrapped).unwrap_or_else(|_| "[]".to_string());
        println!("{json}");
    } else {
        let json =
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_string());
        println!("{json}");
    }
}

pub fn print_csv(results: &[Value], fields: Option<&[String]>) -> Result<(), String> {
    if results.is_empty() {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Determine headers: use specified fields or collect all keys
    let headers: Vec<String> = if let Some(fields) = fields {
        fields.to_vec()
    } else {
        let mut hdrs: Vec<String> = Vec::new();
        for result in results {
            let obj = unwrap_map(result);
            if let Some(obj) = obj {
                for key in obj.keys() {
                    if !hdrs.contains(key) {
                        hdrs.push(key.clone());
                    }
                }
            }
        }
        hdrs
    };

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&headers)
        .map_err(|e| format!("CSV error: {e}"))?;

    for result in results {
        let obj = unwrap_map(result);
        let row: Vec<String> = headers
            .iter()
            .map(|h| {
                obj.and_then(|o| o.get(h))
                    .map(value_to_string)
                    .unwrap_or_default()
            })
            .collect();
        wtr.write_record(&row)
            .map_err(|e| format!("CSV error: {e}"))?;
    }

    let data = wtr.into_inner().map_err(|e| format!("CSV error: {e}"))?;
    handle
        .write_all(&data)
        .map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

pub fn print_raw(results: &[Value]) {
    for msg in results {
        let raw = unwrap_map(msg)
            .and_then(|o| o.get("_raw"))
            .map(value_to_string)
            .unwrap_or_default();
        println!("{raw}");
    }
}

pub fn print_fields(results: &[Value], fields: &[String]) {
    if results.is_empty() {
        eprintln!("No results found.");
        return;
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.load_preset(comfy_table::presets::NOTHING);

    let headers: Vec<String> = fields.iter().map(|f| f.to_uppercase()).collect();
    table.set_header(&headers);

    for result in results {
        let obj = unwrap_map(result);
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
        if let Some(dt) =
            DateTime::<Utc>::from_timestamp(ms / 1000, ((ms % 1000) * 1_000_000) as u32)
        {
            return dt.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        }
    }
    s.to_string()
}
