use crate::trace;
use rustix::net::sockopt::{set_tcp_keepcnt, set_tcp_keepidle, set_tcp_keepintvl};
use std::{os::fd::AsFd, time::Duration};

pub fn enable_tcp_keep_alive(fd: &impl AsFd) -> std::io::Result<()> {
    trace!("Configuring TCP keepalive");

    // start probing after one second of idle
    set_tcp_keepidle(&fd, Duration::from_secs(1))?;

    // retry once a second
    set_tcp_keepintvl(&fd, Duration::from_secs(1))?;

    // die after 3 failed probes (i.e. after 3s of inactivity)
    set_tcp_keepcnt(&fd, 3)?;
    Ok(())
}
