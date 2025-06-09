use crate::stream_id::StreamId;
use futures_util::{SinkExt, Stream};
use mpclipboard_common::Clip;
use std::{collections::HashMap, pin::Pin, task::Poll};
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};

type Ws = WebSocketStream<TcpStream>;

pub(crate) struct MapOfStreams {
    map: HashMap<StreamId, Ws>,
}

impl MapOfStreams {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, id: String, stream: Ws) {
        self.map.insert(StreamId(id), stream);
    }

    pub(crate) fn remove(&mut self, stream_id: &StreamId) -> Ws {
        self.map.remove(stream_id).expect("stream_id must be valid")
    }

    pub(crate) async fn broadcast(&mut self, clip: Clip) {
        let mut ids_to_drop = vec![];

        for (id, conn) in self.map.iter_mut() {
            if let Err(err) = conn.send(Message::from(&clip)).await {
                log::error!("[{id}] failed to broadcast clip: {err:?}");
                ids_to_drop.push(id.clone());
            }
        }

        for id in ids_to_drop {
            self.remove(&id);
        }
    }
}

impl Stream for MapOfStreams {
    type Item = (StreamId, Message);

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut ids_to_drop = vec![];

        let mut out: Poll<Option<Self::Item>> = Poll::Pending;

        for (id, stream) in self.map.iter_mut() {
            // SAFETY:
            // we hold a mutable reference to `self` and `self.map`, so `stream` taken out of it is also uniq
            let stream = unsafe { Pin::new_unchecked(stream) };

            match stream.poll_next(cx) {
                Poll::Ready(Some(Ok(polled))) => {
                    out = Poll::Ready(Some((id.clone(), polled)));
                    break;
                }
                Poll::Ready(Some(Err(err))) => {
                    log::error!("[{id}] {err:?}");
                    ids_to_drop.push(id.clone());
                }
                Poll::Ready(None) => {
                    ids_to_drop.push(id.clone());
                }
                Poll::Pending => {}
            }
        }

        for id in ids_to_drop {
            self.remove(&id);
        }

        out
    }
}
