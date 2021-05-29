use std::fs;
use std::fs::Metadata;
use std::io;
use std::io::{Error, ErrorKind};
use std::fmt;
use std::fmt::Formatter;
use std::path::Path;

pub struct File<'a> {
    pub path: &'a Path,
    pub metadata: Metadata,
    pub format: Format,
    pub handle: fs::File   
}

impl <'a> File<'a> {
    pub fn open(path_str: &String) -> io::Result<File> {
        let path = Path::new(path_str);
        let metadata = path.metadata()?;

        // Directories aren't supported
        if metadata.is_dir() {
            return Err(Error::new(ErrorKind::InvalidInput, ExtFileError::IsDir));
        }

        let format = Format::from(path)?;
        let handle = fs::File::open(path)?;
   
        return Ok(File {
            path,
            handle,
            metadata,
            format,
        });
    }
}

pub enum Format {
    Mpeg
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

#[derive(fmt::Debug)]
enum ExtFileError {
    IsDir, BadExt
}

impl fmt::Display for ExtFileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let msg = match self {
            ExtFileError::IsDir => "Is directory",
            ExtFileError::BadExt => "Could not recognize file extension",
        };

        write!(f, "{}", msg)
    }
}

impl std::error::Error for ExtFileError {}