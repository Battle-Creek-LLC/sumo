# sumo

A fast, minimal CLI for querying [Sumo Logic](https://www.sumologic.com/) logs from the terminal.

```
$ sumo search 'error | count by _sourceCategory' -f -24h

 _SOURCECATEGORY  _COUNT
 prod/app/worker  392
 prod/app/web     47
 prod/app/celery  23
```

## Features

- **Simple search** — query logs with familiar Sumo Logic syntax
- **Multiple output formats** — text tables, JSON, CSV, or raw log lines
- **Keychain auth** — credentials stored securely in macOS Keychain
- **Multiple projects** — switch between Sumo Logic accounts
- **Agent-friendly** — designed for use with AI agents (`-o json -q`)
- **Pipeline-ready** — `--raw` mode outputs one log line per message

## Install

### From source

```bash
cargo install --path .
```

### Build manually

```bash
cargo build --release
cp target/release/sumo /usr/local/bin/
```

## Quick Start

```bash
# 1. Store your credentials
sumo auth login

# 2. Search for errors in the last 24 hours
sumo search 'error' -f -24h

# 3. Count errors by source
sumo search 'error | count by _sourceCategory' -f -24h

# 4. Export as JSON
sumo search 'error' -f -1h -o json -q
```

## Usage

### Search

```bash
sumo search [OPTIONS] <QUERY>
```

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--from` | `-f` | `-15m` | Start time (relative: `-24h`, `-7d`, `-2w` or ISO 8601) |
| `--to` | `-t` | `now` | End time |
| `--output` | `-o` | `text` | Output format: `text`, `json`, `csv` |
| `--limit` | `-l` | `100` | Max results (max 10000) |
| `--fields` | | (all) | Comma-separated fields to include |
| `--raw` | `-r` | | Output raw log lines only |
| `--quiet` | `-q` | | Suppress progress output |
| `--project` | `-p` | (active) | Use a specific project's credentials |

### Examples

```bash
# Keyword search
sumo search 'error' -f -24h

# Filter by source
sumo search '_sourceCategory=prod/app/worker error' -f -24h

# Aggregate query
sumo search 'error | count by _sourceCategory | sort by _count desc' -f -7d

# Time bucketing
sumo search 'error | timeslice 1h | count by _timeslice' -f -24h -o json -q

# Raw logs piped to grep
sumo search '_sourceCategory=prod/app/worker ERROR' -f -1h --raw -q | grep "Traceback"

# CSV export
sumo search 'error | count by _sourceCategory' -f -7d -o csv -q > errors.csv

# Specific fields as JSON
sumo search 'error' -f -24h -o json -q --fields '_messagetime,_sourcecategory,_raw'
```

### Authentication

Credentials are stored in the macOS Keychain.

```bash
# Interactive setup (prompts for deployment, access ID, access key)
sumo auth login

# Scripted setup
sumo auth login --endpoint https://api.us2.sumologic.com/api \
  --access-id YOUR_ID --access-key YOUR_KEY

# Multiple accounts
sumo auth login --project prod
sumo auth login --project staging
sumo auth use prod
sumo auth list

# Check current credentials
sumo auth status
```

Environment variables (`SUMO_ACCESS_ID`, `SUMO_ACCESS_KEY`, `SUMO_API_ENDPOINT`) are supported as a fallback for CI/scripts.

### Other Commands

```bash
# Check a running search job
sumo status <JOB_ID>

# Cancel a running search job
sumo cancel <JOB_ID>
```

## Query Syntax

Queries use [Sumo Logic search syntax](https://help.sumologic.com/docs/search/). Single-quote queries in shell to protect pipes and wildcards.

| Pattern | Example |
|---------|---------|
| Source filter | `_sourceCategory=prod/app/*` |
| Keyword (AND implicit) | `error Traceback` |
| Exact phrase | `"connection refused"` |
| OR | `error OR warning` |
| NOT | `error NOT "health check"` |
| Count | `error \| count by _sourceCategory` |
| Time buckets | `error \| timeslice 1h \| count by _timeslice` |
| Parse fields | `"status=" \| parse "status=*" as status \| count by status` |
| Top N | `error \| count by _sourceCategory \| top 10 _sourceCategory by _count` |

## For AI Agents

Use `-o json -q` for clean, parseable output:

```bash
sumo search 'error | count by _sourceCategory' -f -24h -o json -q
```

Start broad with aggregations, then drill into specific sources and error messages.

## Requirements

- macOS (uses Keychain for credential storage)
- A Sumo Logic account with API access
- Rust toolchain (for building from source)

## License

[MIT](LICENSE)
