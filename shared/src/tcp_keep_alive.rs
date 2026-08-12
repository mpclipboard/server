use crate::trace;
use anyhow::{Context, Result};
use rustix::net::sockopt::{set_tcp_keepcnt, set_tcp_keepidle, set_tcp_keepintvl};
use std::{os::fd::AsFd, time::Duration};

pub fn enable_tcp_keep_alive(fd: &impl AsFd) -> Result<()> {
    trace!("Configuring TCP keepalive");

    // start probing after one second of idle
    set_tcp_keepidle(&fd, Duration::from_secs(1)).context("failed to set TCP_KEEPIDLE")?;

    // retry once a second
    set_tcp_keepintvl(&fd, Duration::from_secs(1)).context("failed to set TCP_KEEPINTVL")?;

    // die after 3 failed probes (i.e. after 3s of inactivity)
    set_tcp_keepcnt(&fd, 3).context("failed to set TCP_KEEPCNT")?;
    Ok(())
}
