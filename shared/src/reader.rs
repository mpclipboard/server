use crate::{
    byte_stream::{ByteStream, ReadResult},
    readbuf::Readbuf,
    trace,
};
use std::os::fd::AsFd;

#[derive(Debug, Clone, Copy)]
pub struct Reader<const N: usize> {
    readbuf: Readbuf<N>,
}

impl<const N: usize> Reader<N> {
    pub fn new() -> Self {
        Self {
            readbuf: Readbuf::new(),
        }
    }

    pub fn new_with_data(data: &[u8]) -> (Self, Option<ReaderResult<N>>) {
        assert!(data.len() <= N);

        if data.len() == N {
            let buf = data.try_into().unwrap_or_else(|_| unreachable!());
            return (Self::new(), Some(ReaderResult::Data(buf)));
        }

        (
            Self {
                readbuf: Readbuf::new_with_data(data),
            },
            None,
        )
    }

    pub fn read_from(&mut self, stream: &mut impl ByteStream, fd: &impl AsFd) -> ReaderResult<N> {
        loop {
            match stream.read_bytes(fd, self.readbuf.remainder()) {
                ReadResult::Data(len) => {
                    trace!("received {len} bytes");
                    if let Some(buf) = self.readbuf.received(len) {
                        return ReaderResult::Data(buf);
                    }
                }
                ReadResult::Eof => return ReaderResult::Died(ReaderError::EOF),
                ReadResult::WouldBlock => return ReaderResult::StillPending,
                ReadResult::Err(err) => return ReaderResult::Died(ReaderError::Transport(err)),
            }
        }
    }
}

#[derive(Debug)]
pub enum ReaderResult<const N: usize> {
    Data([u8; N]),
    StillPending,
    Died(ReaderError),
}

pub enum ReaderError {
    EOF,
    Transport(anyhow::Error),
}

impl std::fmt::Debug for ReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EOF => write!(f, "EOF"),
            Self::Transport(err) => f.debug_tuple("Transport").field(err).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        NonEmptyInlineString,
        byte_stream::{ByteStream, WriteResult},
        message::Message,
    };
    use core::num::NonZeroUsize;
    use std::os::fd::AsFd;

    struct Stream<'a> {
        data: &'a [u8],
    }

    impl ByteStream for Stream<'_> {
        fn read_bytes(&mut self, _fd: &impl AsFd, buf: &mut [u8]) -> ReadResult {
            if self.data.is_empty() {
                return ReadResult::WouldBlock;
            }

            let len = self.data.len().min(buf.len());
            buf.get_mut(..len)
                .unwrap_or_else(|| unreachable!())
                .copy_from_slice(self.data.get(..len).unwrap_or_else(|| unreachable!()));
            self.data = self.data.get(len..).unwrap_or_else(|| unreachable!());

            ReadResult::Data(NonZeroUsize::new(len).unwrap_or_else(|| unreachable!()))
        }

        fn write_bytes(&mut self, _fd: &impl AsFd, _buf: &[u8]) -> WriteResult {
            unreachable!()
        }
    }

    fn message(text: &str) -> Message {
        Message::new(NonEmptyInlineString::new(text).unwrap_or_else(|| unreachable!()))
    }

    fn encoded(message: Message) -> [u8; Message::BYTESIZE] {
        message.encode()
    }

    #[test]
    fn complete_leftover_is_returned_immediately() {
        let message = message("hello");
        let buf = encoded(message);

        let (_, res) = Reader::<{ Message::BYTESIZE }>::new_with_data(&buf);

        match res {
            Some(ReaderResult::Data(data)) => assert_eq!(data, buf),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn partial_leftover_is_preserved() {
        let message = message("hello");
        let buf = encoded(message);
        let (partial, rest) = buf.split_at(Message::BYTESIZE - 1);
        let (mut reader, res) = Reader::<{ Message::BYTESIZE }>::new_with_data(partial);
        let mut stream = Stream { data: rest };
        let fd = std::io::stdin();

        assert!(res.is_none());
        match reader.read_from(&mut stream, &fd) {
            ReaderResult::Data(data) => assert_eq!(data, buf),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn complete_leftover_is_not_decoded() {
        let buf = [0; Message::BYTESIZE];
        let (_, res) = Reader::<{ Message::BYTESIZE }>::new_with_data(&buf);

        match res {
            Some(ReaderResult::Data(data)) => assert_eq!(data, buf),
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
