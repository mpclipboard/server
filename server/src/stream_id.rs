#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StreamId(pub(crate) String);

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
