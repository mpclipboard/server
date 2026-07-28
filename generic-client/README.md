# `MPClipboard`, shared and generic part

This is a shared part of all apps that implement `MPClipboard`'s communication protocol.

It has both Rust and C APIs.

### API

API is purely IO-driven:

1. setup an instance of `MPClipboard`
2. take a file descriptor out of it
3. throw it into **your own** event loop
4. once it's readable

### Usage in Rust

```rust,ignore
use mpclipboard_generic_client::{Config, ConfigReadOption, MPClipboard, Output};

// first initialize a library (this configures a logger and TLS)
MPClipboard::init()?;

// then load a config by providing URI + token + name
let config = Config::new(uri, token, name)?;
// or by reading it from a config.toml in the $CWD
let config = Config::read(ConfigReadOption::FromLocalFile)?;
// or by reading it from $XDG_CONFIG_HOME/mpclipboard/config.toml
let config = Config::read(ConfigReadOption::FromXdgConfigDir)?;

// create an instance of MPClipboard
let mut mpclipboard = MPClipboard::new(config);
// and take its file descriptor
let fd = mpclipboard.as_raw_fd()

loop {
    // FD becomes readable when there's work to do.
    // You can use literally any polling mechanism (e.g. select/poll/epoll/kqueue/io_uring/iocp)
    // to wait until FD becomes readable.
    somehow_wait_readable(fd);
    let output: Output = mpclipboard.read();

    // `output` may contain:
    // 1. received clip (either UTF-8 text or binary blob)
    // 2. information connectivity (connected/connecting/disconnected)
    println!("{:?}", output);
}
```

### Usafe in C

C API fully mirrors Rust API

```c
mpclipboard_init();

mpclipboard_Config *config = mpclipboard_config_read(MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE);
assert(config);

mpclipboard_MPClipboard *mpclipboard = mpclipboard_new(config);
assert(mpclipboard);

int fd = mpclipboard_get_fd(mpclipboard);

for (;;) {
    somehow_wait_readable(fd);
    mpclipboard_Output output = mpclipboard_read(mpclipboard);

    // do something with output (tagged union)
}
```
