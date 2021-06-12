use std::error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, Metadata};
use std::io::{self, Error, ErrorKind};
use std::path::Path;

pub struct File {
    path: String,
    metadata: Metadata,
    format: Format,
    pub handle: fs::File,
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

    pub(crate) fn format(&self) -> &Format {
        &self.format
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
            ExtFileError::IsDir => "Is directory",
            ExtFileError::UnknownExt => "Could not recognize file extension",
        };

        write!(f, "{}", msg)
    }
}

impl error::Error for ExtFileError {}
