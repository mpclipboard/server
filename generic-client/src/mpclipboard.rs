use crate::{Output, config::Config, connection::Connection, logger::Logger, tls::TLS};
use anyhow::{Context, Result};
use mpclipboard_shared::{
    EventLoop, EventLoopResult, Message, NonEmptyInlineString, Store, error, info, trace,
};
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    time::Duration,
};

pub struct MPClipboard {
    event_loop: EventLoop,
    now: u64,
    conn: Connection,
    store: Store,
}

impl MPClipboard {
    pub fn init() -> Result<()> {
        Logger::init();
        TLS::init()?;
        Ok(())
    }

    fn new(config: Config) -> Result<Self> {
        info!("Running with config {config:?}");
        let mut event_loop = EventLoop::new().context("event loop has crashed")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| unreachable!("time goes backwards"))
            .as_secs();
        let conn = Connection::new(config);

        event_loop
            .sync(conn.wants())
            .context("failed to update connection fd in event loop")?;

        Ok(Self {
            event_loop,
            now,
            conn,
            store: Store::empty(),
        })
    }

    pub fn new_inline(url: &str, token: &str, id: &str) -> Result<Self> {
        let config = Config::new(url, token, id)?;
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

    pub fn read(&mut self) -> Result<Option<Output>> {
        let polled = self
            .event_loop
            .wait(Some(Duration::from_secs(0)))
            .context("failed to wait() on event loop")?;

        let prev_connectivity = self.conn.connectivity();
        let message = if let Some(message) = self.drain(&polled)
            && self.store.add(message)
        {
            Some(message.text_as_str().to_string())
        } else {
            None
        };
        let next_connectivity = self.conn.connectivity();

        self.event_loop
            .sync(self.conn.wants())
            .context("failed to update connection fd in event loop")?;

        let connectivity = if prev_connectivity == next_connectivity {
            None
        } else {
            Some(next_connectivity)
        };

        Ok(match (connectivity, message) {
            (Some(connectivity), Some(text)) => Some(Output::Both { connectivity, text }),
            (Some(connectivity), None) => Some(Output::ConnectivityChanged { connectivity }),
            (None, Some(text)) => Some(Output::NewText { text }),
            (None, None) => None,
        })
    }

    fn drain(&mut self, polled: &EventLoopResult) -> Option<Message> {
        let mut out = None;

        if let Some(time) = polled.time {
            self.now = time;
            trace!("tick {}", self.now);
            self.conn.tick(self.now);
        }

        if let Some((readable, writable, has_error)) = polled.fd {
            if has_error && !self.conn.is_disconnected() {
                error!("poll() returned connection error, disconnecting");
                self.conn.disconnect(self.now);
            }

            if readable && !self.conn.is_disconnected() {
                out = self.conn.on_readable(self.now);
            }

            if writable && !self.conn.is_disconnected() {
                self.conn.on_writable(self.now);
            }
        }

        out
    }

    pub fn push_text(&mut self, text: &str) -> bool {
        let Some(text) = NonEmptyInlineString::truncate(text) else {
            info!("Skipping empty text");
            return false;
        };
        let message = Message::new(text);

        if self.store.add(message) {
            self.conn.push(message)
        } else {
            false
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
