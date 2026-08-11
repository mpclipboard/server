use crate::{Output, config::Config, connection::Connection, logger::Logger};
use anyhow::{Context, Result};
use mpclipboard_shared::{
    NonEmptyInlineString, error,
    event_loop::{EventLoop, EventLoopResult},
    info,
    messaging::message::Message,
    store::Store,
    trace,
};
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    time::Duration,
};

/// The main entrypoint
pub struct MPClipboard {
    event_loop: EventLoop,
    now: u64,
    conn: Connection,
    store: Store,
}

impl MPClipboard {
    /// Initializes `MPClipboard`, must be called once at the start of the program.
    /// Internally initializes logger and TLS.
    ///
    /// # Errors
    ///
    /// Returns an error if TLS initialization fails.
    pub fn init() -> Result<()> {
        Logger::init();
        // TLS::init()?;
        Ok(())
    }

    fn new(config: Config) -> Result<Self> {
        info!("Running with config {config:?}");
        let mut event_loop = EventLoop::new()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| unreachable!("time goes backwards"))
            .as_secs();
        let conn = Connection::new(config)?;

        let wants = conn.wants();
        event_loop.sync(wants.conn, wants.heartbeat)?;

        Ok(Self {
            event_loop,
            now,
            conn,
            store: Store::empty(),
        })
    }

    pub fn new_inline(main_url: &str, heartbeat_url: &str, token: &str, id: &str) -> Result<Self> {
        let config = Config::new(main_url, heartbeat_url, token, id)?;
        Self::new(config)
    }

    pub fn new_with_local_config() -> Result<Self> {
        let config = Config::read_local_file()?;
        Self::new(config)
    }

    pub fn new_with_local_config_and_id_override(id: &str) -> Result<Self> {
        let mut config = Config::read_local_file()?;
        config.id = NonEmptyInlineString::new(id).context("malformed id override")?;
        Self::new(config)
    }

    pub fn new_with_xdg_config() -> Result<Self> {
        let config = Config::read_in_xdg_config_dir()?;
        Self::new(config)
    }

    /// Reads data from the connection, returns Output
    ///
    /// # Errors
    ///
    /// Returns an error if OS-specific event loop (epoll/kqueue) returns an error
    pub fn read(&mut self) -> Result<Option<Output>> {
        let mut output = None;
        let polled = self.event_loop.wait(Some(Duration::from_secs(0)))?;

        let prev_connectivity = self.conn.connectivity();
        if let Some(message) = self.drain(polled)
            && self.store.add(message)
        {
            output = Some(Output::NewText {
                text: message.text_as_str().to_string(),
            })
        }
        let next_connectivity = self.conn.connectivity();

        let wants = self.conn.wants();
        self.event_loop.sync(wants.conn, wants.heartbeat)?;

        if prev_connectivity != next_connectivity {
            Ok(Some(Output::ConnectivityChanged {
                connectivity: next_connectivity,
            }))
        } else {
            Ok(output)
        }
    }

    fn drain(&mut self, polled: EventLoopResult) -> Option<Message> {
        let mut out = None;

        if let Some(time) = polled.time {
            self.now = time;
            trace!("tick {}", self.now);
            self.conn.tick(self.now);
        }

        let [conn, heartbeat] = [polled.fd1, polled.fd2];

        if let Some((readable, writable, has_error)) = heartbeat {
            if has_error && !self.conn.is_disconnected() {
                error!("poll() returned heartbeat error, disconnecting");
                self.conn.disconnect(self.now);
            }

            if writable && !self.conn.is_disconnected() {
                self.conn.on_heartbeat_writable(self.now);
            }

            if readable && !self.conn.is_disconnected() {
                self.conn.on_heartbeat_readable(self.now);
            }
        }

        if let Some((readable, writable, has_error)) = conn {
            if has_error && !self.conn.is_disconnected() {
                error!("poll() returned connection error, disconnecting");
                self.conn.disconnect(self.now);
            }

            if readable && !self.conn.is_disconnected() {
                out = self.conn.on_main_conn_readable(self.now);
            }

            if writable && !self.conn.is_disconnected() {
                self.conn.on_main_conn_writable(self.now);
            }
        }

        out
    }

    /// Pushes a new text Clip with provided content.
    /// There's NO queue internally, so this this method overrides previously pushed-but-not-sent Clip.
    ///
    /// # Errors
    ///
    /// Returns an error if OS-specific event loop (epoll/kqueue) returns an error
    pub fn push_text(&mut self, text: &str) -> Result<bool> {
        let Some(text) = NonEmptyInlineString::truncate(text) else {
            error!("Skipping empty text");
            return Ok(false);
        };
        let message = Message::new(text);

        if self.store.add(message) {
            Ok(self.conn.push(message))
        } else {
            Ok(false)
        }
    }
}

impl AsRawFd for MPClipboard {
    fn as_raw_fd(&self) -> i32 {
        self.event_loop.as_raw_fd()
    }
}

impl AsFd for MPClipboard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.event_loop.as_fd()
    }
}
