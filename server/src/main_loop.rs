use crate::{
    as_poll_fd::AsPollFd,
    client::{Client, ClientResult},
    config::Config,
    fd_set::FdSet,
    heartbeat::{Heartbeat, HeartbeatResult},
    pre_sink::{PreSink, PreSinkResult},
    pre_source::{PreSource, PreSourceResult},
    revents::REvents,
    tcp_listener::TcpListener,
};
use anyhow::{Context, Result};
use mpclipboard_shared::{
    ID, Timerfd,
    heartbeat::{HeartbeatRequest, HeartbeatResponse},
    info,
    messaging::{
        handshake::{HandshakeRequest, HandshakeResponse},
        message::Message,
    },
    trace,
};
use rustix::event::PollFlags;
use std::{
    collections::HashMap,
    os::fd::{AsFd, AsRawFd},
};

type MessagingPreSource = PreSource<{ HandshakeRequest::BYTESIZE }, HandshakeRequest>;
type MessagingPreSourceResult = PreSourceResult<{ HandshakeRequest::BYTESIZE }, HandshakeRequest>;

type MessagingPreSink = PreSink<{ HandshakeResponse::BYTESIZE }, HandshakeResponse>;
type MessagingPreSinkResult = PreSinkResult<{ HandshakeResponse::BYTESIZE }, HandshakeResponse>;

type HeartbeatPreSource = PreSource<{ HeartbeatRequest::BYTESIZE }, HeartbeatRequest>;
type HeartbeatPreSourceResult = PreSourceResult<{ HeartbeatRequest::BYTESIZE }, HeartbeatRequest>;

type HeartbeatPreSink = PreSink<{ HeartbeatResponse::BYTESIZE }, HeartbeatResponse>;
type HeartbeatPreSinkResult = PreSinkResult<{ HeartbeatResponse::BYTESIZE }, HeartbeatResponse>;

pub struct MainLoop {
    timer: Timerfd,
    now: u64,
    config: Config,

    messaging_listener: TcpListener,
    messaging_pre_sources: FdSet<20, MessagingPreSource>,
    messaging_pre_sinks: FdSet<20, MessagingPreSink>,
    messaging_clients: FdSet<20, Client>,

    heartbeat_listener: TcpListener,
    heartbeat_pre_sources: FdSet<20, HeartbeatPreSource>,
    heartbeat_pre_sinks: FdSet<20, HeartbeatPreSink>,
    heartbeats: FdSet<20, Heartbeat>,
}

impl MainLoop {
    pub(crate) fn new(config: Config) -> Result<Self> {
        let tcp_listener = TcpListener::new(config.main_url.resolve()?)?;
        let heartbeat_listener = TcpListener::new(config.heartbeat_url.resolve()?)?;

        let timer = Timerfd::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("time goes backwards")?
            .as_secs();
        trace!("start time: {now}");

        let messaging_pre_sources = FdSet::<20, MessagingPreSource>::new();
        let messaging_pre_sinks = FdSet::<20, MessagingPreSink>::new();
        let messaging_clients = FdSet::<20, Client>::new();

        let heartbeat_pre_sources = FdSet::<20, HeartbeatPreSource>::new();
        let heartbeat_pre_sinks = FdSet::<20, HeartbeatPreSink>::new();
        let heartbeats = FdSet::<20, Heartbeat>::new();

        Ok(Self {
            messaging_listener: tcp_listener,
            heartbeat_listener,

            timer,
            now,
            config,

            messaging_pre_sources,
            messaging_pre_sinks,
            messaging_clients,

            heartbeat_pre_sources,
            heartbeat_pre_sinks,
            heartbeats,
        })
    }

    fn poll(&self) -> HashMap<i32, PollFlags> {
        let mut pollfds = core::iter::empty()
            // timer
            .chain(core::iter::once(self.timer.as_poll_fd()))
            // messaging
            .chain(core::iter::once(self.messaging_listener.as_poll_fd()))
            .chain(self.messaging_pre_sources.as_poll_fds())
            .chain(self.messaging_pre_sinks.as_poll_fds())
            .chain(self.messaging_clients.as_poll_fds())
            // heartbeat
            .chain(core::iter::once(self.heartbeat_listener.as_poll_fd()))
            .chain(self.heartbeat_pre_sources.as_poll_fds())
            .chain(self.heartbeat_pre_sinks.as_poll_fds())
            .collect::<Vec<_>>();
        rustix::event::poll(&mut pollfds, None)
            .unwrap_or_else(|err| unreachable!("failed to poll: {err:?}"));

        pollfds
            .into_iter()
            .map(|pollfd| (pollfd.as_fd().as_raw_fd(), pollfd.revents()))
            .collect()
    }

    pub(crate) fn poll_and_process_events(&mut self) {
        let revents = self.poll();

        for (fd, revents) in revents {
            if fd == self.timer.as_raw_fd() {
                self.on_timer_event(revents);
            } else
            // messaging
            if fd == self.messaging_listener.as_raw_fd() {
                self.on_messaging_listener_event(revents);
            }
            if let Some(source) = self.messaging_pre_sources.remove(fd) {
                self.on_messaging_pre_source_event(source, revents);
            } else if let Some(sink) = self.messaging_pre_sinks.remove(fd) {
                self.on_messaging_pre_sink_event(sink, revents);
            } else if let Some(client) = self.messaging_clients.remove(fd) {
                self.on_messaging_client_event(client, revents);
            } else
            // heartbeat
            if fd == self.heartbeat_listener.as_raw_fd() {
                self.on_heartbeat_listener_event(revents);
            } else if let Some(source) = self.heartbeat_pre_sources.remove(fd) {
                self.on_heartbeat_pre_source_event(source, revents);
            } else if let Some(sink) = self.heartbeat_pre_sinks.remove(fd) {
                self.on_heartbeat_pre_sink_event(sink, revents);
            }
        }
    }

    fn on_timer_event(&mut self, revents: PollFlags) {
        let revents = REvents::new(revents)
            .unwrap_or_else(|err| unreachable!("failed to poll() timerfd: {err:?}"));

        if !revents.readable {
            return;
        }

        self.now = self.timer.read();
        trace!("tick {}", self.now);

        self.send_heartbeats();

        self.messaging_pre_sources.reap(self.now);
        self.messaging_pre_sinks.reap(self.now);
        self.heartbeat_pre_sources.reap(self.now);
        self.heartbeat_pre_sinks.reap(self.now);
    }

    fn on_messaging_listener_event(&mut self, revents: PollFlags) {
        let fd = match self.messaging_listener.accept(revents) {
            Ok(Some(fd)) => fd,
            Ok(None) => return,
            Err(err) => unreachable!("failed to accept(): {err:?}"),
        };
        let source = MessagingPreSource::new(fd, self.now);
        trace!("new {source}");
        self.messaging_pre_sources.insert(source);
    }

    fn on_messaging_pre_source_event(&mut self, source: MessagingPreSource, revents: PollFlags) {
        match source.on_poll_event(revents, self.now) {
            MessagingPreSourceResult::Died => {}
            MessagingPreSourceResult::StillPending(source) => {
                self.messaging_pre_sources.insert(source);
            }
            MessagingPreSourceResult::Done((req, fd)) => {
                let id = req.id;
                if req.token == self.config.token {
                    let sink = MessagingPreSink::new(fd, id, self.now, &HandshakeResponse);
                    info!("promoting {id} to {sink}");
                    self.messaging_pre_sinks.insert(sink);
                } else {
                    info!("auth failed for {id}");
                }
            }
        }
    }

    fn on_messaging_pre_sink_event(&mut self, sink: MessagingPreSink, revents: PollFlags) {
        match sink.on_poll_event(revents, self.now) {
            MessagingPreSinkResult::Died => {}
            MessagingPreSinkResult::StillPending(sink) => {
                self.messaging_pre_sinks.insert(sink);
            }
            MessagingPreSinkResult::Done((id, fd)) => {
                let client = Client::new(fd, id);
                info!("promoting {id} to {client}");
                self.messaging_clients.insert(client);
            }
        }
    }

    fn on_messaging_client_event(&mut self, client: Client, revents: PollFlags) {
        match client.on_poll_event(revents) {
            ClientResult::Died => {}
            ClientResult::Message((message, client)) => {
                info!("broadcasting {message:?}");
                self.broadcast(&message, client.id());

                self.messaging_clients.insert(client);
            }
            ClientResult::StillPending(client) => {
                self.messaging_clients.insert(client);
            }
        }
    }

    fn on_heartbeat_listener_event(&mut self, revents: PollFlags) {
        let fd = match self.heartbeat_listener.accept(revents) {
            Ok(Some(fd)) => fd,
            Ok(None) => return,
            Err(err) => unreachable!("failed to accept(): {err:?}"),
        };
        let source = HeartbeatPreSource::new(fd, self.now);
        trace!("new {source}");
        self.heartbeat_pre_sources.insert(source);
    }

    fn on_heartbeat_pre_source_event(&mut self, source: HeartbeatPreSource, revents: PollFlags) {
        match source.on_poll_event(revents, self.now) {
            HeartbeatPreSourceResult::Died => {}
            HeartbeatPreSourceResult::StillPending(source) => {
                self.heartbeat_pre_sources.insert(source);
            }
            HeartbeatPreSourceResult::Done((req, fd)) => {
                if !self.has_client(req.id) {
                    return;
                }

                let sink = HeartbeatPreSink::new(fd, req.id, self.now, &HeartbeatResponse);
                info!("promoting {} to {sink}", req.id);
                self.heartbeat_pre_sinks.insert(sink);
            }
        }
    }

    fn on_heartbeat_pre_sink_event(&mut self, sink: HeartbeatPreSink, revents: PollFlags) {
        match sink.on_poll_event(revents, self.now) {
            HeartbeatPreSinkResult::Died => {}
            HeartbeatPreSinkResult::StillPending(sink) => {
                self.heartbeat_pre_sinks.insert(sink);
            }
            HeartbeatPreSinkResult::Done((id, fd)) => {
                let heartbeat = Heartbeat::new(fd, id);
                info!("heartbeat started for {id}");
                self.heartbeats.insert(heartbeat);
            }
        }
    }

    fn has_client(&self, id: ID) -> bool {
        self.messaging_clients.fds().any(|client| client.id() == id)
    }

    fn broadcast(&mut self, message: &Message, sender_id: ID) {
        self.messaging_clients
            .fds_mut()
            .filter(|client| client.id() != sender_id)
            .for_each(|client| client.push(message));
    }

    fn send_heartbeats(&mut self) {
        let mut fds_to_drop = vec![];
        for (fd, heartbeat) in self.heartbeats.iter_mut() {
            match heartbeat.tick(self.now) {
                HeartbeatResult::Died => fds_to_drop.push(*fd),
                HeartbeatResult::Ok => {}
            }
        }
        for fd in fds_to_drop {
            self.heartbeats.remove(fd);
        }
    }
}
