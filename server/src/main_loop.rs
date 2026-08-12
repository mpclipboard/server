use crate::{
    as_poll_fd::AsPollFd,
    client::{Client, ClientResult},
    config::Config,
    fd_set::FdSet,
    pre_sink::{PreSink, PreSinkResult},
    pre_source::{PreSource, PreSourceResult},
    tcp_listener::TcpListener,
};
use anyhow::{Context, Result};
use mpclipboard_shared::{
    ID, Timerfd, error, handshake_request::HandshakeRequestParser,
    handshake_response::HandshakeResponse, info, message::Message,
    revents::REvents,
    store::Store,
    tcp_keep_alive::enable_tcp_keep_alive,
    trace,
};
use rustix::event::PollFlags;
use std::{
    collections::HashMap,
    os::fd::{AsFd, AsRawFd},
};

type MessagingPreSource = PreSource<HandshakeRequestParser>;
type MessagingPreSourceResult = PreSourceResult<HandshakeRequestParser>;

type MessagingPreSink = PreSink<{ HandshakeResponse::BYTESIZE }, HandshakeResponse>;
type MessagingPreSinkResult = PreSinkResult<{ HandshakeResponse::BYTESIZE }, HandshakeResponse>;

pub struct MainLoop {
    timer: Timerfd,
    now: u64,
    config: Config,
    store: Store,

    listener: TcpListener,
    pre_sources: FdSet<20, MessagingPreSource>,
    pre_sinks: FdSet<20, MessagingPreSink>,
    clients: FdSet<20, Client>,
}

impl MainLoop {
    pub(crate) fn new(config: Config) -> Result<Self> {
        let listener = TcpListener::new(config.url.resolve()?)?;

        let timer = Timerfd::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("time goes backwards")?
            .as_secs();
        trace!("start time: {now}");

        let pre_sources = FdSet::<20, MessagingPreSource>::new();
        let pre_sinks = FdSet::<20, MessagingPreSink>::new();
        let clients = FdSet::<20, Client>::new();

        Ok(Self {
            listener,
            timer,
            now,
            config,
            store: Store::empty(),

            pre_sources,
            pre_sinks,
            clients,
        })
    }

    fn poll(&self) -> HashMap<i32, PollFlags> {
        let mut pollfds = core::iter::empty()
            .chain(core::iter::once(self.timer.as_poll_fd()))
            .chain(core::iter::once(self.listener.as_poll_fd()))
            .chain(self.pre_sources.as_poll_fds())
            .chain(self.pre_sinks.as_poll_fds())
            .chain(self.clients.as_poll_fds())
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
            } else if fd == self.listener.as_raw_fd() {
                self.on_listener_event(revents);
            }
            if let Some(source) = self.pre_sources.remove(fd) {
                self.on_pre_source_event(source, revents);
            } else if let Some(sink) = self.pre_sinks.remove(fd) {
                self.on_pre_sink_event(sink, revents);
            } else if let Some(client) = self.clients.remove(fd) {
                self.on_client_event(client, revents);
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

        self.pre_sources.reap(self.now);
        self.pre_sinks.reap(self.now);
    }

    fn on_listener_event(&mut self, revents: PollFlags) {
        let fd = match self.listener.accept(revents) {
            Ok(Some(fd)) => fd,
            Ok(None) => return,
            Err(err) => unreachable!("failed to accept(): {err:?}"),
        };
        let source = MessagingPreSource::new(fd, self.now);
        trace!("new {source}");
        self.pre_sources.insert(source);
    }

    fn on_pre_source_event(&mut self, source: MessagingPreSource, revents: PollFlags) {
        match source.on_poll_event(revents, self.now) {
            MessagingPreSourceResult::Died => {}
            MessagingPreSourceResult::StillPending(source) => {
                self.pre_sources.insert(source);
            }
            MessagingPreSourceResult::Done((req, fd)) => {
                let id = req.id;
                if req.token == self.config.token {
                    let sink = MessagingPreSink::new(fd, id, self.now, &HandshakeResponse);
                    info!("promoting {id} to {sink}");
                    self.pre_sinks.insert(sink);
                } else {
                    info!("auth failed for {id}: {req:?}");
                }
            }
        }
    }

    fn on_pre_sink_event(&mut self, sink: MessagingPreSink, revents: PollFlags) {
        match sink.on_poll_event(revents, self.now) {
            MessagingPreSinkResult::Died => {}
            MessagingPreSinkResult::StillPending(sink) => {
                self.pre_sinks.insert(sink);
            }
            MessagingPreSinkResult::Done((id, fd)) => match enable_tcp_keep_alive(&fd) {
                Ok(()) => {
                    let mut client = Client::new(fd, id);
                    info!("promoting {id} to {client}");
                    if let Some(message) = self.store.current() {
                        client.push(&message);
                    }
                    self.clients.insert(client);
                }
                Err(err) => error!("{err:?}"),
            },
        }
    }

    fn on_client_event(&mut self, client: Client, revents: PollFlags) {
        match client.on_poll_event(revents) {
            ClientResult::Died => {}
            ClientResult::Message((message, client)) => {
                if self.store.add(message) {
                    info!("broadcasting {message:?}");
                    self.broadcast(&message, client.id());
                }

                self.clients.insert(client);
            }
            ClientResult::StillPending(client) => {
                self.clients.insert(client);
            }
        }
    }

    fn broadcast(&mut self, message: &Message, sender_id: ID) {
        self.clients
            .fds_mut()
            .filter(|client| client.id() != sender_id)
            .for_each(|client| client.push(message));
    }
}
