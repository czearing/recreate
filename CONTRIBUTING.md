# Contributing

## Setup

```bash
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Pull requests

- Keep changes focused.
- Add regression coverage for behavior changes.
- Preserve structured evidence and restoration paths.
- Run formatting, Clippy, tests, and the release build.
- Use Conventional Commit titles (`feat:`, `fix:`, `docs:`).

Do not commit captured sites, credentials, browser profiles, or generated
evidence.
