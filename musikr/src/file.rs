use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, Metadata};
use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

use crate::id3v2;

pub struct File {
    metadata: Metadata,
    _format: Format,
    handle: fs::File,
}

impl File {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        let metadata = path.as_ref().metadata()?;

        // Directories aren't supported
        if metadata.is_dir() {
            return Err(Error::new(ErrorKind::InvalidInput, ExtFileError::IsDir));
        }

        let format = Format::new(path.as_ref())?;
        let handle = fs::File::open(path)?;

        Ok(File {
            metadata,
            _format: format,
            handle,
        })
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn handle(&mut self) -> &mut fs::File {
        &mut self.handle
    }

    pub fn id3v2(&mut self) -> io::Result<id3v2::Tag> {
        id3v2::search(self)
    }

    pub(crate) fn seek(&mut self, to: u64) -> io::Result<u64> {
        self.handle.seek(SeekFrom::Start(to))
    }

    pub(crate) fn read_into(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.handle.read_exact(buf)
    }

    pub(crate) fn read_up_to(&mut self, amount: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; amount];
        let n = self.handle.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Format {
    Mpeg,
}

impl Format {
    fn new(path: &Path) -> io::Result<Format> {
        if let Some(ext) = path.extension() {
            if ext == "mp3" {
                return Ok(Format::Mpeg);
            }
        }

        // Any unknown or nonexistant extensions are treated as Unknown
        Err(Error::new(
            ErrorKind::InvalidInput,
            ExtFileError::UnknownExt,
        ))
    }
}

#[derive(Debug)]
enum ExtFileError {
    IsDir,
    UnknownExt,
}

impl Display for ExtFileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let msg = match self {
            ExtFileError::IsDir => "Is a directory",
            ExtFileError::UnknownExt => "Could not recognize file extension",
        };

        write!(f, "{}", msg)
    }
}

impl error::Error for ExtFileError {}
