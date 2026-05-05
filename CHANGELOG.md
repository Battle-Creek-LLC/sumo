# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- Bump `openssl` 0.10.76 → 0.10.79 and `rustls-webpki` 0.103.9 → 0.103.13
  via `cargo update`. Closes nine open Dependabot advisories surfaced when
  the repository was migrated to the Battle-Creek-LLC org and Dependabot
  was enabled:
  - [GHSA-hppc-g8h3-xhp3][] — high — `rust-openssl`: unchecked callback
    length in PSK / cookie trampolines leaks adjacent memory to peer.
  - [GHSA-ghm9-cr32-g9qj][] — high — `rust-openssl`:
    `MdCtxRef::digest_final()` writes past caller buffer with no length check.
  - [GHSA-8c75-8mhr-p7r9][] — high — `rust-openssl`: incorrect bounds
    assertion in AES key wrap.
  - [GHSA-pqf5-4pqq-29f5][] — high — `rust-openssl`: `Deriver::derive` and
    `PkeyCtxRef::derive` can overflow short buffers on OpenSSL 1.1.1.
  - [GHSA-xmgf-hq76-4vx2][] — low — `rust-openssl`: out-of-bounds read in
    PEM password callback when returning an oversized length.
  - [GHSA-82j2-j2ch-gfr8][] — high — `rustls-webpki`: denial of service via
    panic on malformed CRL `BIT STRING`.
  - [GHSA-pwjx-qhcg-rvj4][] — medium — `rustls-webpki`: CRLs not considered
    authoritative by Distribution Point due to faulty matching logic.
  - [GHSA-xgp8-3hg3-c2mh][] — low — `rustls-webpki`: name constraints
    accepted for certificates asserting a wildcard name.
  - [GHSA-965h-392x-2mh5][] — low — `rustls-webpki`: name constraints for
    URI names incorrectly accepted.

### Changed

- Replace direct `security-framework` dependency with the cross-platform
  [`keyring`](https://crates.io/crates/keyring) crate (features
  `apple-native`, `linux-native`, `sync-secret-service`). sumo now builds
  and runs on macOS, Linux, and Windows; previously it was macOS-only.
  Service / account naming (`com.sumologic.cli.{project}.{key}` /
  `sumo-cli`) is preserved so existing macOS Keychain entries continue to
  resolve. On headless Linux without a Secret Service daemon, fall back to
  the existing `SUMO_ACCESS_ID` / `SUMO_ACCESS_KEY` / `SUMO_API_ENDPOINT`
  environment variables.

[GHSA-hppc-g8h3-xhp3]: https://github.com/advisories/GHSA-hppc-g8h3-xhp3
[GHSA-ghm9-cr32-g9qj]: https://github.com/advisories/GHSA-ghm9-cr32-g9qj
[GHSA-8c75-8mhr-p7r9]: https://github.com/advisories/GHSA-8c75-8mhr-p7r9
[GHSA-pqf5-4pqq-29f5]: https://github.com/advisories/GHSA-pqf5-4pqq-29f5
[GHSA-xmgf-hq76-4vx2]: https://github.com/advisories/GHSA-xmgf-hq76-4vx2
[GHSA-82j2-j2ch-gfr8]: https://github.com/advisories/GHSA-82j2-j2ch-gfr8
[GHSA-pwjx-qhcg-rvj4]: https://github.com/advisories/GHSA-pwjx-qhcg-rvj4
[GHSA-xgp8-3hg3-c2mh]: https://github.com/advisories/GHSA-xgp8-3hg3-c2mh
[GHSA-965h-392x-2mh5]: https://github.com/advisories/GHSA-965h-392x-2mh5

## [0.1.0] — 2026-03-17

First public release. macOS-only Sumo Logic search CLI with Keychain-backed
credential storage.

### Added

- `sumo search` with relative (`-24h`, `-7d`, `-2w`) and ISO 8601 time
  windows; `text`, `json`, and `csv` output formats; `--raw` mode for
  pipe-friendly log lines; `--fields` for column projection; `--limit`
  up to 10000.
- `sumo auth login`, `auth logout`, `auth use`, `auth list`, `auth status`
  for managing one or more named projects (multi-account support).
- `sumo status <job-id>` and `sumo cancel <job-id>` for inspecting and
  aborting in-flight search jobs.
- Environment-variable fallback (`SUMO_ACCESS_ID`, `SUMO_ACCESS_KEY`,
  `SUMO_API_ENDPOINT`) for CI and headless contexts.
