use futures_util::Stream;
use std::{collections::HashMap, pin::Pin, task::Poll};

pub(crate) struct SelectAllIdentified<T, S>
where
    S: Stream<Item = T> + Unpin,
{
    map: HashMap<StreamId, Pin<Box<S>>>,
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

impl<T, S> SelectAllIdentified<T, S>
where
    S: Stream<Item = T> + Unpin,
{
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, uuid: impl Into<String>, stream: S) {
        self.map.insert(StreamId(uuid.into()), Box::pin(stream));
    }

    pub(crate) fn remove(&mut self, stream_id: &StreamId) -> S {
        let value = self.map.remove(stream_id).expect("stream_id must be valid");
        *Pin::into_inner(value)
    }
}

impl<T, S> Stream for SelectAllIdentified<T, S>
where
    S: Stream<Item = T> + Unpin,
{
    type Item = (StreamId, T);

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
