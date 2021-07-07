use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, ErrorKind};
use std::ops::Range;

/// A simple ergonomics layer around an internal slice, created primarily to automate bounds checking.
pub struct BufStream<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> BufStream<'a> {
    /// Construct a new `BufStream` from `src`.
    pub fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    /// Read this stream into a buffer. If the end of a stream is reached, then the
    /// remaining bytes will be unchanged.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let len = usize::min(self.remaining(), buf.len());
        buf.copy_from_slice(&self.src[self.pos..self.pos + len]);
        self.pos += len;
        len
    }

    /// Read this stream into a buffer. If the buffer cannot be completely filled, then
    /// an error will be returned. The buffer will be in an indeterminate state if this operation
    /// fails.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if self.remaining() < buf.len() {
            return Err(underread_error());
        }

        buf.copy_from_slice(&self.src[self.pos..self.pos + buf.len()]);
        self.pos += buf.len();

        Ok(())
    }

    /// Read this stream into an array of size `N`, returning an error when the slice
    /// cannot be filled.
    pub fn read_array<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut arr = [0; N];
        self.read_exact(&mut arr)?;
        Ok(arr)
    }

    /// Read exactly one byte from this stream. If there is no data remaining in the stream then
    /// an error will be returned.
    pub fn read_u8(&mut self) -> io::Result<u8> {
        if self.is_empty() {
            return Err(eos_error());
        }

        self.pos += 1;

        Ok(self.src[self.pos - 1])
    }

    /// Read a big-endian u32 from this stream. If the u32 cannot be filled an error will be returned.
    pub fn read_u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(self.read_array()?))
    }

    /// Read a big-endian u32 from this stream. If the u32 cannot be filled an error will be returned.
    pub fn read_u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    /// Read a big-endian u64 from this stream. If the u64 cannot be filled an error will be returned.
    pub fn read_u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    /// Read a big-endian i16 from this stream. If the i16 cannot be filled an error will be returned.
    pub fn read_i16(&mut self) -> io::Result<i16> {
        Ok(i16::from_be_bytes(self.read_array()?))
    }

    /// Skip `n` bytes in this stream. If this skip is beyond the stream length then an error will be
    /// returned.
    pub fn skip(&mut self, n: usize) -> io::Result<()> {
        if self.remaining() < n {
            return Err(oob_error());
        }

        self.pos += n;

        Ok(())
    }

    /// Consumes the stream and returns a slice of size n. If the slice cannot be created, then an error is returned.
    pub fn slice(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.remaining() < n {
            return Err(underread_error());
        }

        self.pos += n;

        Ok(&self.src[self.pos - n..self.pos])
    }

    /// Like `BufStream::slice`, but it returns a self-contained BufStream of the slice.
    pub fn slice_stream(&mut self, n: usize) -> io::Result<BufStream> {
        Ok(BufStream::new(self.slice(n)?))
    }

    /// Peek at a portion of this stream relative to the current position. This does not consume the stream.
    /// If the peek location is out of bounds, an error will be returned.
    pub fn peek(&self, range: Range<usize>) -> io::Result<&[u8]> {
        let start = range.start + self.pos;
        let end = range.end + self.pos;

        if start > self.len() || end > self.len() {
            return Err(oob_error());
        }

        Ok(&self.src[start..end])
    }

    /// Searches for `needle` and returns a slice of the data including the pattern.
    /// If the pattern cannot be found, the remaining buffer is returned. If the stream is
    /// consumed, then it will return an error.
    /// This function will consume the stream until either one of those conditions are met.
    pub fn search(&mut self, needle: &[u8]) -> &[u8] {
        let start = self.pos;
        let limit = self.pos + self.remaining();

        let mut begin = self.pos;
        let mut end = self.pos + needle.len();

        while end <= limit {
            if &self.src[begin..end] == needle {
                self.pos = end;

                return &self.src[start..self.pos - needle.len()];
            }

            begin += needle.len();
            end += needle.len();
        }

        self.take_rest()
    }

    /// Takes the rest of the streams data into a slice, leaving the stream in an fully consumed state.
    pub fn take_rest(&mut self) -> &[u8] {
        let rest = &self.src[self.pos..];
        self.pos += self.remaining();
        rest
    }

    /// Returns the length of this stream
    pub fn len(&self) -> usize {
        self.src.len()
    }

    /// Returns the stream position
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the remaining length of this stream.
    pub fn remaining(&self) -> usize {
        self.len() - self.pos()
    }

    /// Returns if this stream has been fully consumed.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }
}

#[derive(Debug)]
pub enum StreamError {
    EndOfStream,
    BufferUnderread,
    OutOfBounds,
}

impl Display for StreamError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write![f, "{:?}", self]
    }
}

impl error::Error for StreamError {
    // Nothing to implement
}

#[inline(always)]
fn eos_error() -> io::Error {
    io::Error::new(ErrorKind::UnexpectedEof, StreamError::EndOfStream)
}

#[inline(always)]
fn underread_error() -> io::Error {
    io::Error::new(ErrorKind::UnexpectedEof, StreamError::BufferUnderread)
}

#[inline(always)]
fn oob_error() -> io::Error {
    io::Error::new(ErrorKind::UnexpectedEof, StreamError::OutOfBounds)
}
