# Contributing

Thanks for your interest in contributing to sumo!

## Development

```bash
# Clone and build
git clone https://github.com/jstockdi/sumo.git
cd sumo
cargo build

# Run tests
cargo test

# Run locally
cargo run -- search 'error' -f -1h
```

## Submitting Changes

1. Fork the repository
2. Create a feature branch (`git checkout -b my-feature`)
3. Make your changes
4. Run `cargo test` and `cargo clippy`
5. Commit and push
6. Open a pull request

## Reporting Issues

Open an issue on GitHub with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- `sumo --help` output and OS version
