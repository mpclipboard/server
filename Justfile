server:
    RUST_LOG=trace cargo run --bin shared-clipboard-server -- --start

generate-config:
    RUST_LOG=trace cargo run --bin shared-clipboard-server -- --generate

client name url token:
    RUST_LOG=trace cargo run --bin shared-clipboard-test-client -- {{name}} "{{url}}" "{{token}}"

now := `date +%s`

build-deb-packages:
    cargo build --target=x86_64-unknown-linux-musl --release

    strip target/x86_64-unknown-linux-musl/release/shared-clipboard-server
    cargo deb --deb-revision="{{now}}" -p shared-clipboard-server --target=x86_64-unknown-linux-musl

    strip target/x86_64-unknown-linux-musl/release/shared-clipboard-test-client
    cargo deb --deb-revision="{{now}}" -p shared-clipboard-test-client --target=x86_64-unknown-linux-musl
