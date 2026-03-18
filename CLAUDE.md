# Project Guidelines

## After Every Code Change

Run clippy and format checks before considering a change complete:

```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

If either fails, fix the issues before proceeding.
