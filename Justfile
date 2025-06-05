run:
    RUST_LOG=trace cargo run -- --start

generate-config:
    RUST_LOG=trace cargo run -- --generate

now := `date +%s`

build-deb-package:
    cargo build --target=x86_64-unknown-linux-musl --release

    strip target/x86_64-unknown-linux-musl/release/mpclipboard-server
    cargo deb --deb-revision="{{now}}" -p mpclipboard-server --target=x86_64-unknown-linux-musl
