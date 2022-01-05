/// Shared Tag IO.
use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::OpenOptions;
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;

/// An ergonomics layer around a byte slice.
///
/// This is meant to automate bounds checking and data transformation when parsing tags.
/// Any tag that allows custom parsers will provide this type to an implementation.
#[derive(Clone)]
pub struct BufStream<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> BufStream<'a> {
    /// Construct a new `BufStream` from `src`.
    pub(crate) fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    /// Reads this stream into a buffer.
    ///
    /// If the end of a stream is reached, then the remaining bytes will be unchanged.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let len = usize::min(self.remaining(), buf.len());
        buf[..len].copy_from_slice(&self.src[self.pos..self.pos + len]);
        self.pos += len;
        len
    }

    /// Reads this stream into a buffer.
    ///
    /// # Errors
    /// If this buffer cannot be filled, then an error will be returned.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if self.remaining() < buf.len() {
            return Err(underread_error(buf.len(), self.remaining()));
        }

        buf.copy_from_slice(&self.src[self.pos..self.pos + buf.len()]);
        self.pos += buf.len();

        Ok(())
    }

    /// Reads this stream into an array of size `N`.
    ///
    /// # Errors
    /// If the array cannot be filled, then an error is returned.
    pub fn read_array<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut arr = [0; N];
        self.read_exact(&mut arr)?;
        Ok(arr)
    }

    /// Reads exactly one [`u8`](u8) from this stream.
    ///
    /// # Errors
    /// If the stream is exhausted, then an error will be returned.
    pub fn read_u8(&mut self) -> io::Result<u8> {
        if self.is_empty() {
            return Err(eos_error());
        }

        self.pos += 1;

        Ok(self.src[self.pos - 1])
    }

    /// Reads exactly [`i8`](i8) from this stream.
    ///
    /// # Errors
    /// If the stream is exhausted, then an error will be returned.
    pub fn read_i8(&mut self) -> io::Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    /// Reads a big-endian [`u16`](u16) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_be_u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(self.read_array()?))
    }

    /// Reads a big-endian [`u32`](u32) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_be_u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    /// Reads a big-endian [`u64`](u64) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_be_u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    /// Reads a big-endian [`i16`](i16) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_be_i16(&mut self) -> io::Result<i16> {
        Ok(i16::from_be_bytes(self.read_array()?))
    }

    /// Reads a big-endian [`i32`](i32) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_be_i32(&mut self) -> io::Result<i32> {
        Ok(i32::from_be_bytes(self.read_array()?))
    }

    /// Reads a big-endian [`i64`](i64) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_be_i64(&mut self) -> io::Result<i64> {
        Ok(i64::from_be_bytes(self.read_array()?))
    }

    /// Reads a little-endian [`u16`](u16) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_le_u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    /// Reads a little-endian [`u32`](u32) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_le_u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    /// Reads a little-endian [`u64`](u64) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_le_u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }

    /// Reads a little-endian [`i16`](i16) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_le_i16(&mut self) -> io::Result<i16> {
        Ok(i16::from_le_bytes(self.read_array()?))
    }

    /// Reads a little-endian [`i32`](i32) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_le_i32(&mut self) -> io::Result<i32> {
        Ok(i32::from_le_bytes(self.read_array()?))
    }

    /// Reads a little-endian [`i64`](i64) from this stream.
    ///
    /// # Errors
    /// If the there is not enough bytes to construct the type, then an error is returned.
    pub fn read_le_i64(&mut self) -> io::Result<i64> {
        Ok(i64::from_le_bytes(self.read_array()?))
    }

    /// Skips `n` bytes in this stream.
    ///
    /// # Errors
    /// If this skip is beyond the stream length, then an error will be returned.
    pub fn skip(&mut self, n: usize) -> io::Result<()> {
        if self.remaining() < n {
            return Err(oob_error(self.pos + n, self.len()));
        }

        self.pos += n;

        Ok(())
    }

    /// Consumes the stream and returns a slice of size n.
    ///
    /// # Errors
    /// If the slice bounds are outside of the range of the stream, then an error is returned.
    pub fn slice(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.remaining() < n {
            return Err(underread_error(n, self.remaining()));
        }

        self.pos += n;

        Ok(&self.src[self.pos - n..self.pos])
    }

    /// Like [`slice`](BufStream::slice), but it returns a new `BufStream` containing the slice.
    ///
    /// # Errors
    /// If the slice bounds are outside of the range of the stream, then an error is returned.
    pub fn slice_stream(&mut self, n: usize) -> io::Result<BufStream> {
        Ok(BufStream::new(self.slice(n)?))
    }

    /// Peeks at a portion of this stream relative to the current position, without consuming the stream.
    ///
    /// # Errors
    /// If the peek location is out of bounds, an error will be returned.
    pub fn peek(&self, range: Range<usize>) -> io::Result<&[u8]> {
        let start = range.start + self.pos;
        let end = range.end + self.pos;

        if start > self.len() {
            return Err(oob_error(start, self.len()));
        }

        if end > self.len() {
            return Err(oob_error(end, self.len()));
        }

        Ok(&self.src[start..end])
    }

    /// Searches for `needle` and returns a slice of the data including the pattern.
    ///
    /// This function will consume the stream until the stream is exhausted or if the
    /// pattern has been found. It will then return all of the data it consumed while
    /// searching. If the stream is exhausted, nothing is returned.
    pub fn search(&mut self, needle: &[u8]) -> &[u8] {
        let start = self.pos;
        let limit = self.pos + self.remaining();

        let mut begin = self.pos;
        let mut end = self.pos + needle.len();

        while end <= limit {
            if &self.src[begin..end] == needle {
                self.pos = end;

                return &self.src[start..self.pos];
            }

            begin += needle.len();
            end += needle.len();
        }

        self.take_rest()
    }

    /// Consumes the rest of the stream into a slice, exhausting the stream.
    pub fn take_rest(&mut self) -> &[u8] {
        let rest = &self.src[self.pos..];
        self.pos += self.remaining();
        rest
    }

    /// Copies the entire buffer of this stream into a [`Vec`](std::vec::Vec),
    /// without consuming the stream.
    pub fn to_vec(&self) -> Vec<u8> {
        self.src.to_vec()
    }

    /// Returns the length of this stream.
    pub fn len(&self) -> usize {
        self.src.len()
    }

    /// Returns the stream position.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the remaining length of this stream.
    pub fn remaining(&self) -> usize {
        self.len() - self.pos()
    }

    /// Returns if this stream is exhausted.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }
}

/// The error type returned when a [`BufStream`](BufStream) read fails.
#[derive(Debug, Clone)]
pub enum StreamError {
    /// The stream was exhausted.
    EndOfStream,
    /// The buffer was left unread.
    BufferUnderread {
        len: usize,
        remaining: usize,
    },
    OutOfBounds {
        pos: usize,
        len: usize,
    },
}

impl Display for StreamError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            StreamError::EndOfStream => write![f, "end of stream"],
            StreamError::BufferUnderread { len, remaining } => write![
                f,
                "buffer underread: length is {} but stream only has {}",
                len, remaining
            ],
            StreamError::OutOfBounds { pos, len } => {
                write![f, "out of bounds: index is {} but length is {}", pos, len]
            }
        }
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
fn underread_error(len: usize, remaining: usize) -> io::Error {
    io::Error::new(
        ErrorKind::UnexpectedEof,
        StreamError::BufferUnderread { len, remaining },
    )
}

#[inline(always)]
fn oob_error(pos: usize, len: usize) -> io::Error {
    io::Error::new(
        ErrorKind::UnexpectedEof,
        StreamError::OutOfBounds { pos, len },
    )
}

/// Replace up to `end` bytes in a file with `data`.
pub fn write_replaced<P: AsRef<Path>>(path: P, data: &[u8], end: u64) -> io::Result<()> {
    match data.len() as u64 {
        len if len == end => {
            let mut file = OpenOptions::new().create(true).write(true).open(&path)?;

            // The lengths match, we can just blit directly.
            file.write_all(data)?;
            file.flush()
        }

        _ => {
            // The lengths do not match, read the rest of the file and then re-blit all
            // the data in sequence. This isn't efficent, but theres noting else we can do.
            let keep = match OpenOptions::new().read(true).open(&path) {
                Ok(mut file) => {
                    let mut keep = Vec::with_capacity(file.stream_position()? as usize);
                    file.seek(SeekFrom::Start(end))?;
                    file.read_to_end(&mut keep)?;
                    keep
                }

                Err(_) => Vec::new(),
            };

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;

            file.write_all(data)?;
            file.write_all(&keep)?;
            file.flush()
        }
    }
}
