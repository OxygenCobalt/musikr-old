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

        let format = Format::from(path)?;
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

    pub(super) fn format(&self) -> &Format {
        &self.format
    }
}

pub(super) enum Format {
    Mpeg,
}

impl Format {
    fn from(path: &Path) -> io::Result<Format> {
        if let Some(ext) = path.extension() {
            if ext == "mp3" {
                return Ok(Format::Mpeg);
            }
        }

        // Any unknown or nonexistant extensions are treated as Unknown
        return Err(Error::new(ErrorKind::InvalidInput, ExtFileError::BadExt));
    }
}

#[derive(Debug)]
enum ExtFileError {
    IsDir,
    BadExt,
}

impl Display for ExtFileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let msg = match self {
            ExtFileError::IsDir => "Is directory",
            ExtFileError::BadExt => "Could not recognize file extension",
        };

        write!(f, "{}", msg)
    }
}

impl std::error::Error for ExtFileError {}
