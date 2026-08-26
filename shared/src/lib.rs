#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::std_instead_of_core)]
#![expect(clippy::missing_errors_doc)]
#![expect(clippy::missing_panics_doc)]
#![doc = include_str!("../README.md")]

mod config;
pub use config::ConfigParser;

mod http_lines_buffer;
mod readbuf;
mod writebuf;

mod http_lines_reader;
mod reader;
mod writer;

mod handshake_request;
pub use handshake_request::HandshakeRequest;

mod handshake_request_reader;
pub use handshake_request_reader::HandshakeRequestReader;

mod handshake_request_writer;
pub use handshake_request_writer::HandshakeRequestWriter;

mod handshake_response;

mod handshake_response_reader;
pub use handshake_response_reader::HandshakeResponseReader;

mod handshake_response_writer;
pub use handshake_response_writer::HandshakeResponseWriter;

mod message;
pub use message::Message;

mod message_reader;
pub use message_reader::MessageReader;

mod message_writer;
pub use message_writer::MessageWriter;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod timerfd;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use timerfd::Timerfd;

pub(crate) const MAX_HOST_LENGTH: usize = 256 + 1 + 5;
pub type Host = NonEmptyInlineString<MAX_HOST_LENGTH>;

pub(crate) const MAX_TOKEN_LENGTH: usize = 100;
pub type Token = NonEmptyInlineString<MAX_TOKEN_LENGTH>;

pub(crate) const MAX_ID_LENGTH: usize = 100;
pub type ID = NonEmptyInlineString<MAX_ID_LENGTH>;

pub(crate) const START_LINE: &str = "GET / HTTP/1.1";
pub(crate) const HOST_PREFIX: &str = "Host: ";
pub(crate) const TOKEN_PREFIX: &str = "Token: ";
pub(crate) const ID_PREFIX: &str = "ID: ";
pub(crate) const CONNECTION_UPGRADE_HEADER: &str = "Connection: Upgrade";
pub(crate) const UPGRADE_MPCLIPBOARD_RAW_HEADER: &str = "Upgrade: mpclipboard-raw";
pub(crate) const PADDING_PREFIX: &str = "Padding: ";
pub(crate) const MIN_PADDING_LENGTH: usize = 1;

mod non_empty_inline_string;
pub use non_empty_inline_string::NonEmptyInlineString;

mod wants;
pub use wants::Wants;

mod event_loop;
pub use event_loop::{EventLoop, EventLoopResult};

#[doc(hidden)]
pub mod logger;

mod revents;
pub use revents::REvents;

mod store;
pub use store::Store;

mod tcp_keep_alive;
pub use tcp_keep_alive::enable_tcp_keep_alive;

mod url;
pub use url::Url;

pub(crate) fn strip_prefix_ignore_ascii_case<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let (pre, post) = line.split_at_checked(prefix.len())?;
    if pre.eq_ignore_ascii_case(prefix) {
        Some(post)
    } else {
        None
    }
}
