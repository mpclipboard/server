use anyhow::Result;
use mpclipboard_generic_client::{MPClipboard, Output};
use mpclipboard_shared::event_loop::{EventLoop, EventLoopResult, Wants};
use std::{
    io::BufRead,
    os::fd::{AsRawFd as _, BorrowedFd},
};

const HELP: &str = "Usage:
cargo run --example cli -- <periodically sent text>

Example:

RUST_LOG=info cargo run --example cli -- <id>
";
fn print_help_and_exit() -> ! {
    eprintln!("{HELP}");
    std::process::exit(1);
}

fn main() -> Result<()> {
    MPClipboard::init()?;

    let [_, id] = std::env::args()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|_| print_help_and_exit());

    let mut mpclipboard = MPClipboard::new_with_local_config_and_id_override(&id)?;
    let mut stdin = std::io::stdin().lock();

    let mut event_loop = EventLoop::new()?;
    event_loop.sync(
        Some((
            unsafe { BorrowedFd::borrow_raw(mpclipboard.as_raw_fd()) },
            Wants::Read,
        )),
        Some((
            unsafe { BorrowedFd::borrow_raw(stdin.as_raw_fd()) },
            Wants::Read,
        )),
    )?;

    loop {
        let EventLoopResult {
            time: _,
            fd1: mpclipboard_polled,
            fd2: stdin_polled,
        } = event_loop.wait(None)?;

        if let Some((readable, writable, err)) = mpclipboard_polled {
            assert!(!err);
            assert!(!writable);

            if readable {
                if let Some(output) = mpclipboard.read()? {
                    match output {
                        Output::ConnectivityChanged { connectivity } => {
                            println!("{connectivity:?}")
                        }
                        Output::NewText { text } => println!("[{text}]"),
                    }
                }
            }
        }
        if let Some((readable, writable, err)) = stdin_polled {
            assert!(!err);
            assert!(!writable);

            if readable {
                let mut line = String::new();
                stdin.read_line(&mut line)?;
                let line = line.trim();
                if !line.is_empty() {
                    mpclipboard.push_text(line)?;
                }
            }
        }
    }
}
