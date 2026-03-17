# sumo CLI — Specification

A fast, minimal command-line interface for querying Sumo Logic logs.

## Goals

- Query Sumo Logic logs from the terminal with simple, memorable commands
- Support time-range searches (e.g., last 24h) for production monitoring
- Output results in human-readable and machine-parseable formats
- Work well as a building block in shell pipelines and Claude Code commands

## Non-Goals

- Managing collectors, sources, dashboards, or other Sumo Logic resources
- Replacing the Sumo Logic web UI for complex analytics
- Real-time log tailing (use Sumo Logic Live Tail for that)

---

## Authentication

Credentials are stored securely in the macOS Keychain and managed via the `sumo auth` command. Multiple projects (credential sets) are supported.

### Projects

A **project** is a named set of credentials (access ID, access key, endpoint). This allows switching between different Sumo Logic accounts or environments (e.g., `prod`, `staging`, `dev`).

- The `default` project is used when no `--project` flag is provided
- The active project can be switched with `sumo auth use <name>`

### Stored Credentials

Credentials are stored in the macOS Keychain using the project name as a namespace:

| Credential | Keychain Service Name | Description |
|---|---|---|
| Access ID | `com.sumologic.cli.<project>.access-id` | Sumo Logic Access ID |
| Access Key | `com.sumologic.cli.<project>.access-key` | Sumo Logic Access Key |
| API Endpoint | `com.sumologic.cli.<project>.endpoint` | Region-specific API base URL |
| Active Project | `com.sumologic.cli.active-project` | Name of the currently active project |

Authentication uses HTTP Basic Auth (`access_id:access_key`).

### `sumo auth login`

Store credentials in the macOS Keychain. Prompts interactively for each value.

```
sumo auth login [--project <name>]
```

```
Project name [default]: prod
Sumo Logic API Endpoint
  1) US1  https://api.sumologic.com/api
  2) US2  https://api.us2.sumologic.com/api
  3) EU   https://api.eu.sumologic.com/api
  4) AU   https://api.au.sumologic.com/api
  5) JP   https://api.jp.sumologic.com/api
  6) CA   https://api.ca.sumologic.com/api
  7) IN   https://api.in.sumologic.com/api
Select deployment [1-7]: 2
Access ID: su1a2B3cD4eF5g
Access Key: ********
Credentials saved to keychain (project: prod).
```

Options:

| Option | Description |
|---|---|
| `--project <name>` | Project name (default: `default`) |
| `--endpoint <URL>` | Set endpoint directly (skip prompt) |
| `--access-id <ID>` | Set access ID directly (skip prompt) |
| `--access-key <KEY>` | Set access key directly (skip prompt) |

When all options are provided, no interactive prompts are shown. This supports scripted setup.

### `sumo auth logout`

Remove stored credentials for a project from the Keychain.

```
sumo auth logout [--project <name>]
```

```
Credentials removed from keychain (project: prod).
```

Use `--all` to remove credentials for all projects.

### `sumo auth use`

Switch the active project.

```
sumo auth use <name>
```

```
Switched to project: prod
```

### `sumo auth list`

List all configured projects.

```
sumo auth list
```

```
  default   https://api.us2.sumologic.com/api (US2)
* prod      https://api.us2.sumologic.com/api (US2)
  staging   https://api.eu.sumologic.com/api (EU)
```

The active project is marked with `*`.

### `sumo auth status`

Show the current authentication state (access key is masked).

```
sumo auth status
```

```
Project:    prod
Endpoint:   https://api.us2.sumologic.com/api (US2)
Access ID:  su1a2B3c***
Access Key: ****
```

### Project Selection Order

For all commands, the project is resolved in this order:

1. `--project <name>` flag (if provided on any command)
2. Active project set via `sumo auth use`
3. `default` project

### Credential Lookup Order

Once the project is resolved, credentials are looked up:

1. macOS Keychain (primary)
2. Environment variables `SUMO_ACCESS_ID`, `SUMO_ACCESS_KEY`, `SUMO_API_ENDPOINT` (fallback, for CI/scripts — ignores project selection)

If no credentials are found, commands exit with: `Not authenticated. Run 'sumo auth login' to set up credentials.`

---

## Global Options

These options are available on all commands that require authentication (`search`, `status`, `cancel`):

| Option | Short | Default | Description |
|---|---|---|---|
| `--project` | `-p` | (active) | Use credentials from the named project |

---

## Commands

### `sumo search`

Run a log search query and return results.

```
sumo search [OPTIONS] <QUERY>
```

**Arguments:**

| Argument | Description |
|---|---|
| `<QUERY>` | Sumo Logic search query string |

**Options:**

| Option | Short | Default | Description |
|---|---|---|---|
| `--from` | `-f` | `-15m` | Start time (ISO 8601, relative like `-24h`, `-7d`, or `now`) |
| `--to` | `-t` | `now` | End time (same formats as `--from`) |
| `--timezone` | `-z` | `UTC` | Timezone for query |
| `--limit` | `-l` | `100` | Max number of messages to return (max 10000) |
| `--offset` | | `0` | Starting offset for pagination |
| `--output` | `-o` | `text` | Output format: `text`, `json`, `csv` |
| `--fields` | | (all) | Comma-separated list of fields to include |
| `--by-receipt-time` | | `false` | Use receipt time instead of message time |
| `--raw` | `-r` | `false` | Output raw `_raw` field only (one message per line) |
| `--poll-interval` | | `2` | Seconds between status polls |
| `--quiet` | `-q` | `false` | Suppress progress output (status, counts) |

**Relative time format:**

Relative times are expressed as `-<number><unit>` where unit is:
- `s` — seconds
- `m` — minutes
- `h` — hours
- `d` — days
- `w` — weeks

Examples: `-15m`, `-24h`, `-7d`, `-2w`, `-30s`

**Examples:**

```bash
# Errors in the last 24 hours
sumo search "error" -f -24h

# Errors from a specific source category
sumo search '_sourceCategory=prod/plotzy/worker error' -f -24h

# Count errors by source, output as JSON
sumo search 'error | count by _sourceCategory' -f -24h -o json

# Raw log lines only, pipe to grep
sumo search '_sourceCategory=prod/plotzy/worker ERROR' -f -1h --raw | grep "Traceback"

# Specific fields
sumo search 'error' -f -24h --fields '_messagetime,_sourceCategory,_raw'

# CSV export for spreadsheet
sumo search 'error | count by _sourceCategory' -f -7d -o csv > errors.csv
```

### `sumo status`

Check the status of a running search job.

```
sumo status <JOB_ID>
```

Returns the job state, message count, record count, and any pending errors/warnings.

### `sumo cancel`

Cancel a running search job.

```
sumo cancel <JOB_ID>
```

---

## Search Job Lifecycle

The `sumo search` command manages the full Search Job API lifecycle internally:

```
1. POST   /v2/search/jobs          → Create job, get job ID
2. GET    /v2/search/jobs/{id}     → Poll status until DONE GATHERING RESULTS
3. GET    /v2/search/jobs/{id}/messages  → Fetch log messages
   — OR —
   GET    /v2/search/jobs/{id}/records   → Fetch aggregated records (if query has aggregate)
4. DELETE /v2/search/jobs/{id}     → Clean up job
```

**Behavior:**
- Poll every `--poll-interval` seconds (default 2s)
- Show progress on stderr: `Searching... 1,234 messages found` (unless `--quiet`)
- Aggregation detection: if the query contains a pipe (`|`) followed by an aggregation keyword (`count`, `sum`, `avg`, `min`, `max`, `first`, `last`, `pct`, `stddev`, `group by`), fetch records instead of messages
- Auto-paginate: the API returns up to 10,000 results per page; if `--limit` exceeds a single page, fetch additional pages until the limit is reached
- Always clean up (DELETE) the search job when done, even on Ctrl+C
- Keep-alive: if `--poll-interval` exceeds 20s, override to 20s to prevent Sumo Logic session timeout

---

## Output Formats

### `text` (default)

Human-readable table format:

```
TIME                     SOURCE                          MESSAGE
2026-03-17 14:57:25 UTC  prod/plotzy/worker              EmptyResponseError: Empty response from Gemini
2026-03-17 14:57:53 UTC  prod/plotzy/celery              Error finding information for GOMEZ ENTERPRISE: 2 validation errors
```

For aggregated queries:

```
_SOURCECATEGORY              COUNT
prod/plotzy/worker           47
prod/plotzy/celery           23
prod/plotzy/web              5
```

### `json`

Array of objects, one per message/record:

```json
[
  {
    "_messagetime": "2026-03-17T14:57:25Z",
    "_sourceCategory": "prod/plotzy/worker",
    "_raw": "EmptyResponseError: Empty response from Gemini..."
  }
]
```

### `csv`

Standard CSV with header row:

```csv
_messagetime,_sourceCategory,_raw
2026-03-17T14:57:25Z,prod/plotzy/worker,"EmptyResponseError: Empty response from Gemini..."
```

---

## Error Handling

| Condition | Behavior |
|---|---|
| Not authenticated | Exit 1: `Not authenticated. Run 'sumo auth login' to set up credentials.` |
| Project not found | Exit 1: `Project '<name>' not found. Run 'sumo auth list' to see available projects.` |
| 401 Unauthorized | Exit 1: `Authentication failed. Check credentials with 'sumo auth status'.` |
| 429 Rate Limited | Retry with exponential backoff (1s, 2s, 4s), max 3 retries |
| 400 Bad Request | Exit 1 with Sumo Logic error message (e.g., parse error in query) |
| 404 Not Found | Exit 1: `Search job not found (may have expired).` |
| Job state CANCELLED | Exit 1: `Search job was cancelled by the server.` |
| Ctrl+C / SIGINT | Cancel the search job (DELETE), then exit |
| Network error | Retry once after 2s, then exit 1 with error |
| Keychain access denied | Exit 1: `Unable to access macOS Keychain. Check system permissions.` |

---

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Error (auth, query, network, etc.) |
| 2 | Invalid arguments / usage error |

---

## Build & Install

```bash
cargo build --release
cp target/release/sumo /usr/local/bin/
```

Or via cargo:

```bash
cargo install --path .
```

---

## Dependencies (Rust crates)

| Crate | Purpose |
|---|---|
| `clap` | Argument parsing with derive macros |
| `reqwest` | HTTP client (blocking or async) |
| `serde` / `serde_json` | JSON serialization/deserialization |
| `chrono` | Time parsing and relative time calculation |
| `tokio` | Async runtime (if using async reqwest) |
| `security-framework` | macOS Keychain access |
| `dialoguer` | Interactive prompts for auth login |
| `ctrlc` | Graceful Ctrl+C handling for job cleanup |
| `comfy-table` | Text table formatting |
| `csv` | CSV output |

---

## Text Output Behavior

For `text` format output:

- Column widths are auto-sized based on content, up to terminal width
- The `MESSAGE` column is the last column and takes remaining width
- Messages longer than available width are truncated with `...`
- Use `--raw` or `--output json` for full untruncated output

---

## Future Considerations

These are explicitly out of scope for v1 but noted for reference:

- **Saved searches** — run a named/saved query
- **Live tail** — stream logs in real time via WebSocket
- **Shell completions** — generate completions for bash/zsh/fish
- **`--timeout`** — max wait time for a search job before auto-cancelling
- **Colored output** — syntax highlighting for log levels, with `--no-color` flag
- **`--follow` mode** — repeat a query on an interval for monitoring
