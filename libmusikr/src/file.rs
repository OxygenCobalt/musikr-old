use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, Metadata};
use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

use crate::id3v2;

pub struct File {
    path: String,
    metadata: Metadata,
    format: Format,
    handle: fs::File,
}

impl File {
    pub fn open(path_str: &str) -> io::Result<File> {
        let path = Path::new(path_str);
        let metadata = path.metadata()?;

        // Directories aren't supported
        if metadata.is_dir() {
            return Err(Error::new(ErrorKind::InvalidInput, ExtFileError::IsDir));
        }

        let format = Format::new(path)?;
        let handle = fs::File::open(path)?;

        // Don't need to keep around the path instance
        let path = path_str.to_string();

        Ok(File {
            path,
            metadata,
            format,
            handle,
        })
    }

    pub fn path(&self) -> &String {
        &self.path
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

    pub(crate) fn format(&self) -> &Format {
        &self.format
    }

    pub(crate) fn seek(&mut self, to: u64) -> io::Result<u64> {
        self.handle.seek(SeekFrom::Start(to))
    }

    pub(crate) fn read_into(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.handle.read_exact(buf)
    }

    pub(crate) fn read_bytes(&mut self, amount: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; amount];
        self.handle.read_exact(&mut buf)?;
        Ok(buf)
    }
}

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
