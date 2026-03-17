use crate::auth;
use crate::output;
use crate::time;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct SearchArgs {
    pub query: String,
    pub from: String,
    pub to: String,
    pub timezone: String,
    pub limit: u32,
    pub offset: u32,
    pub output: String,
    pub fields: Option<String>,
    pub by_receipt_time: bool,
    pub raw: bool,
    pub poll_interval: u64,
    pub quiet: bool,
    pub project: Option<String>,
}

struct ApiClient {
    client: Client,
    endpoint: String,
    access_id: String,
    access_key: String,
}

impl ApiClient {
    fn new(creds: auth::Credentials) -> Self {
        Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .expect("Failed to create HTTP client"),
            endpoint: creds.endpoint,
            access_id: creds.access_id,
            access_key: creds.access_key,
        }
    }

    fn post_json(&self, path: &str, body: &Value) -> Result<(StatusCode, Value), String> {
        let url = format!("{}{}", self.endpoint, path);
        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.access_id, Some(&self.access_key))
            .json(body)
            .send()
            .map_err(|e| format!("Network error: {e}"))?;
        let status = resp.status();
        let body: Value = resp.json().unwrap_or(Value::Null);
        Ok((status, body))
    }

    fn get_json(&self, path: &str) -> Result<(StatusCode, Value), String> {
        let url = format!("{}{}", self.endpoint, path);
        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.access_id, Some(&self.access_key))
            .send()
            .map_err(|e| format!("Network error: {e}"))?;
        let status = resp.status();
        let body: Value = resp.json().unwrap_or(Value::Null);
        Ok((status, body))
    }

    fn delete(&self, path: &str) -> Result<(), String> {
        let url = format!("{}{}", self.endpoint, path);
        self.client
            .delete(&url)
            .basic_auth(&self.access_id, Some(&self.access_key))
            .send()
            .map_err(|e| format!("Network error: {e}"))?;
        Ok(())
    }
}

fn handle_error_status(status: StatusCode, body: &Value) -> Result<(), String> {
    match status {
        s if s == StatusCode::UNAUTHORIZED => {
            Err("Authentication failed. Check credentials with 'sumo auth status'.".to_string())
        }
        s if s == StatusCode::BAD_REQUEST => {
            let msg = body
                .get("message")
                .and_then(|m| m.as_str())
                .map(|m| m.to_string())
                .unwrap_or_else(|| format!("Bad request: {body}"));
            Err(msg)
        }
        s if s == StatusCode::NOT_FOUND => {
            Err("Search job not found (may have expired).".to_string())
        }
        s if s == StatusCode::TOO_MANY_REQUESTS => {
            Err("rate_limited".to_string()) // sentinel for retry logic
        }
        s if s.is_client_error() || s.is_server_error() => {
            let msg = body
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown API error");
            Err(format!("API error ({s}): {msg}"))
        }
        _ => Ok(()),
    }
}

fn is_aggregate_query(query: &str) -> bool {
    if let Some(pipe_pos) = query.find('|') {
        let after_pipe = &query[pipe_pos + 1..].to_lowercase();
        let keywords = [
            "count", "sum", "avg", "min", "max", "first", "last", "pct", "stddev", "group",
            "timeslice", "top", "sort", "parse", "where", "limit",
        ];
        keywords.iter().any(|kw| after_pipe.contains(kw))
    } else {
        false
    }
}

pub fn run(args: SearchArgs) -> Result<(), String> {
    let creds = auth::resolve_credentials(args.project.as_deref())?;
    let api = ApiClient::new(creds);

    let from_time = time::parse_time(&args.from)?;
    let to_time = time::parse_time(&args.to)?;

    let from_ms = from_time.format("%Y-%m-%dT%H:%M:%S").to_string();
    let to_ms = to_time.format("%Y-%m-%dT%H:%M:%S").to_string();

    // Create search job
    let mut create_body = serde_json::json!({
        "query": args.query,
        "from": from_ms,
        "to": to_ms,
        "timeZone": args.timezone,
        "byReceiptTime": args.by_receipt_time,
    });

    if let Some(obj) = create_body.as_object_mut() {
        obj.insert("autoParsingMode".to_string(), Value::String("AutoParse".to_string()));
    }

    let (status, body) = with_retry(|| api.post_json("/v2/search/jobs", &create_body))?;
    handle_error_status(status, &body)?;

    let job_id = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Failed to get search job ID. Status: {status}, Response: {body}"))?
        .to_string();

    if !args.quiet {
        eprintln!("Search job created: {job_id}");
    }

    // Set up Ctrl+C handler
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();
    let _ = ctrlc::set_handler(move || {
        cancelled_clone.store(true, Ordering::SeqCst);
    });

    // Poll for completion
    let poll_secs = args.poll_interval.min(20); // cap at 20s for keep-alive
    let result = poll_and_fetch(&api, &job_id, &args, poll_secs, &cancelled);

    // Always clean up
    let _ = api.delete(&format!("/v2/search/jobs/{job_id}"));

    result
}

fn poll_and_fetch(
    api: &ApiClient,
    job_id: &str,
    args: &SearchArgs,
    poll_secs: u64,
    cancelled: &Arc<AtomicBool>,
) -> Result<(), String> {
    loop {
        if cancelled.load(Ordering::SeqCst) {
            eprintln!("\nSearch cancelled.");
            return Ok(());
        }

        let (status, body) = with_retry(|| api.get_json(&format!("/v2/search/jobs/{job_id}")))?;
        handle_error_status(status, &body)?;

        let state = body
            .get("state")
            .and_then(|s| s.as_str())
            .unwrap_or("UNKNOWN");

        let msg_count = body
            .get("messageCount")
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        let rec_count = body
            .get("recordCount")
            .and_then(|c| c.as_u64())
            .unwrap_or(0);

        if !args.quiet {
            eprint!("\rSearching... {msg_count} messages, {rec_count} records found");
        }

        match state {
            "DONE GATHERING RESULTS" => {
                if !args.quiet {
                    eprintln!();
                }
                break;
            }
            "CANCELLED" => {
                return Err("Search job was cancelled by the server.".to_string());
            }
            "FORCE PAUSED" => {
                if !args.quiet {
                    eprintln!("\nSearch job force paused. Fetching available results...");
                }
                break;
            }
            _ => {
                thread::sleep(std::time::Duration::from_secs(poll_secs));
            }
        }
    }

    // Determine if aggregate
    let is_agg = is_aggregate_query(&args.query);
    let results = fetch_results(api, job_id, args, is_agg)?;

    // Parse fields filter if provided
    let fields: Option<Vec<String>> = args.fields.as_ref().map(|f| {
        f.split(',').map(|s| s.trim().to_string()).collect()
    });

    // Apply field filtering then format output
    let filtered = if let Some(ref field_list) = fields {
        output::filter_fields(&results, field_list)
    } else {
        results
    };

    if args.raw {
        output::print_raw(&filtered);
    } else {
        match args.output.as_str() {
            "json" => output::print_json(&filtered, fields.is_some()),
            "csv" => output::print_csv(&filtered, fields.as_deref())?,
            _ => {
                if fields.is_some() {
                    output::print_fields(&filtered, fields.as_ref().unwrap());
                } else {
                    output::print_text(&filtered, is_agg);
                }
            }
        }
    }

    Ok(())
}

fn fetch_results(
    api: &ApiClient,
    job_id: &str,
    args: &SearchArgs,
    is_aggregate: bool,
) -> Result<Vec<Value>, String> {
    let endpoint_type = if is_aggregate { "records" } else { "messages" };
    let mut all_results = Vec::new();
    let mut offset = args.offset;
    let limit = args.limit;
    let page_size = 10_000u32;

    loop {
        let remaining = limit.saturating_sub(offset - args.offset);
        if remaining == 0 {
            break;
        }
        let fetch_count = remaining.min(page_size);

        let path = format!(
            "/v2/search/jobs/{job_id}/{endpoint_type}?offset={offset}&limit={fetch_count}"
        );

        let (status, body) = with_retry(|| api.get_json(&path))?;
        handle_error_status(status, &body)?;

        let results_key = if is_aggregate { "records" } else { "messages" };
        let items = body
            .get(results_key)
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        let count = items.len() as u32;
        all_results.extend(items);

        if count < fetch_count {
            break; // no more results
        }
        offset += count;
    }

    Ok(all_results)
}

fn with_retry<F>(mut f: F) -> Result<(StatusCode, Value), String>
where
    F: FnMut() -> Result<(StatusCode, Value), String>,
{
    let delays = [1, 2, 4]; // exponential backoff seconds
    let mut last_err = String::new();

    for (attempt, delay) in std::iter::once(0).chain(delays.into_iter()).enumerate() {
        if attempt > 0 {
            thread::sleep(std::time::Duration::from_secs(delay));
        }

        match f() {
            Ok((status, body)) => {
                if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                    last_err = "Rate limited by Sumo Logic API".to_string();
                    continue;
                }
                return Ok((status, body));
            }
            Err(e) if e.starts_with("Network error") && attempt < 1 => {
                last_err = e;
                thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_err)
}

pub fn job_status(job_id: &str, project: Option<&str>) -> Result<(), String> {
    let creds = auth::resolve_credentials(project)?;
    let api = ApiClient::new(creds);

    let (status, body) = with_retry(|| api.get_json(&format!("/v2/search/jobs/{job_id}")))?;
    handle_error_status(status, &body)?;

    let state = body.get("state").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
    let msg_count = body.get("messageCount").and_then(|c| c.as_u64()).unwrap_or(0);
    let rec_count = body.get("recordCount").and_then(|c| c.as_u64()).unwrap_or(0);

    println!("Job ID:        {job_id}");
    println!("State:         {state}");
    println!("Messages:      {msg_count}");
    println!("Records:       {rec_count}");

    if let Some(errors) = body.get("pendingErrors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            println!("Pending Errors:");
            for err in errors {
                println!("  - {}", err.as_str().unwrap_or("unknown"));
            }
        }
    }

    if let Some(warnings) = body.get("pendingWarnings").and_then(|w| w.as_array()) {
        if !warnings.is_empty() {
            println!("Pending Warnings:");
            for warn in warnings {
                println!("  - {}", warn.as_str().unwrap_or("unknown"));
            }
        }
    }

    Ok(())
}

pub fn cancel_job(job_id: &str, project: Option<&str>) -> Result<(), String> {
    let creds = auth::resolve_credentials(project)?;
    let api = ApiClient::new(creds);

    api.delete(&format!("/v2/search/jobs/{job_id}"))?;
    eprintln!("Search job {job_id} cancelled.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_aggregate_query() {
        assert!(is_aggregate_query("error | count by _sourceCategory"));
        assert!(is_aggregate_query("* | sum(bytes) by host"));
        assert!(is_aggregate_query("error | avg(latency)"));
        assert!(is_aggregate_query("error | group by host"));
        assert!(is_aggregate_query("error | timeslice 1h | count by _timeslice"));
        assert!(is_aggregate_query("error | count by _sourceCategory | top 10 _sourceCategory by _count"));
        assert!(is_aggregate_query("error | count by _sourceCategory | sort by _count desc"));
        assert!(is_aggregate_query("\"status=\" | parse \"status=*\" as status | count by status"));
        assert!(is_aggregate_query("error | count by _sourceCategory | where _count > 100"));
        assert!(!is_aggregate_query("error"));
        assert!(!is_aggregate_query("_sourceCategory=prod error"));
        assert!(!is_aggregate_query("count something")); // no pipe
    }
}
