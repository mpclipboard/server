server:
    RUST_LOG=trace cargo run --bin shared-clipboard-server -- --start

generate-config:
    RUST_LOG=trace cargo run --bin shared-clipboard-server -- --generate

client name url token:
    RUST_LOG=trace cargo run --bin client -- {{name}} "{{url}}" "{{token}}"
