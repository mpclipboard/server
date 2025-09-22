# MPClipboard server

This is a central part in communication process, a server that connects clients of all flavors.

It's based on [Tokio](https://tokio.rs/) and an amazing [`tokio-websockets`](https://github.com/Gelbpunkt/tokio-websockets/) library.

It has only one endpoint: `/` that:

1. receives incoming TCP connections
2. performs initial handshake and switches to WebSocket protocol
3. performs authentication (and disconnects if it fails)
4. sends the most recent clip to newly connected client
5. starts receiving new clipboard texts from clients
6. broadcasts them to all connected clients

Authentication is based on a static token that is written in the `config.toml`.

The server itself doesn't handle any TLS, instead it expects a reverse proxy in front of it (Nginx/caddy/etc).

### Building

```
cargo build --release
```

Additionally, there's a [`debian/mpclipboard-server.service`](/debian/mpclipboard-server.service) systemd service if you need it.
