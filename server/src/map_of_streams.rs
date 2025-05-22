use common::Clip;
use futures_util::{SinkExt, Stream};
use std::{collections::HashMap, pin::Pin, task::Poll};
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};

type Ws = WebSocketStream<TcpStream>;

pub(crate) struct MapOfStreams {
    map: HashMap<StreamId, Pin<Box<Ws>>>,
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StreamId(String);

impl std::fmt::Debug for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl MapOfStreams {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, uuid: impl Into<String>, stream: Ws) {
        self.map.insert(StreamId(uuid.into()), Box::pin(stream));
    }

    pub(crate) fn remove(&mut self, stream_id: &StreamId) -> Ws {
        let value = self.map.remove(stream_id).expect("stream_id must be valid");
        *Pin::into_inner(value)
    }

    pub(crate) async fn broadcast(&mut self, clip: Clip) {
        for (id, conn) in self.map.iter_mut() {
            if let Err(err) = conn.send(Message::from(clip.clone())).await {
                log::error!("[{id}] failed to broadcast clip: {err:?}");
            }
        }
    }
}

impl Stream for MapOfStreams {
    type Item = (StreamId, Result<Message, tokio_websockets::Error>);

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut disconnected = vec![];
        let mut out: Poll<Option<Self::Item>> = Poll::Pending;

        for (stream_id, stream) in self.map.iter_mut() {
            let stream = stream.as_mut();
            match stream.poll_next(cx) {
                Poll::Ready(Some(polled)) => {
                    out = Poll::Ready(Some((stream_id.clone(), polled)));
                    break;
                }
                Poll::Ready(None) => {
                    disconnected.push(stream_id.clone());
                }
                Poll::Pending => {}
            }
        }

        for id in disconnected {
            self.map.remove(&id);
        }

        out
    }
}
