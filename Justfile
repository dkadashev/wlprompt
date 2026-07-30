check: (fmt "--check") clippy test

fmt args="":
    cargo fmt {{args}}

clippy:
    cargo clippy --all-targets

test:
    cargo test
