use crate::clip::Clip;
use futures_util::{SinkExt, Stream, StreamExt};
use pin_project_lite::pin_project;
use std::{collections::HashMap, fmt::Display, hash::Hash, pin::Pin, task::Poll};
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};

type Ws = WebSocketStream<TcpStream>;

pin_project! {
    pub(crate) struct Multiplexer<K>
    {
        #[pin]
        map: HashMap<K, Pin<Box<Ws>>>,
    }
}

impl<K> Multiplexer<K>
where
    K: Hash + Eq + Display + Copy,
{
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, id: K, stream: Ws) {
        self.map.insert(id, Box::pin(stream));
    }

    pub(crate) fn remove(&mut self, id: &K) -> Pin<Box<Ws>> {
        self.map.remove(id).expect("id must be valid")
    }

    pub(crate) async fn broadcast(&mut self, clip: Clip) {
        let mut ids_to_drop = vec![];

        for (id, conn) in self.map.iter_mut() {
            if let Err(err) = conn.send(clip.to_message()).await {
                log::error!("[{id}] failed to broadcast clip: {err:?}");
                ids_to_drop.push(*id);
            }
        }

        for id in ids_to_drop {
            self.remove(&id);
        }
    }
}

impl<K> Stream for Multiplexer<K>
where
    K: Hash + Eq + Display + Copy + Unpin,
{
    type Item = (K, Message);

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let map = self.project().map.get_mut();
        let mut ids_to_drop = vec![];
        let mut out = Poll::Pending;

        for (id, stream) in map.iter_mut() {
            let Poll::Ready(value) = stream.poll_next_unpin(cx) else {
                continue;
            };

            let Some(value) = value else {
                log::error!("[{id}] stream has closed");
                ids_to_drop.push(*id);
                continue;
            };

            match value {
                Ok(value) => {
                    out = Poll::Ready(Some((*id, value)));
                    break;
                }
                Err(err) => {
                    log::error!("[{id}] {err:?}");
                    ids_to_drop.push(*id);
                }
            }
        }

        for id in ids_to_drop {
            map.remove(&id);
        }

        out
    }
}
