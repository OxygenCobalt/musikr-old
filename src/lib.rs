use std::fs;
use std::fs::Metadata;
use std::path::Path;
use std::io;
use std::io::{Error, ErrorKind};

pub struct File<'a> {
    pub path: &'a Path,
    pub handle: fs::File,
    pub metadata: Metadata,
    pub format: Format
}

pub enum Format {
    Mpeg
}

pub fn open(path_str: &String) -> io::Result<File> {
    let path = Path::new(path_str);
    let metadata = validate_path(path)?;
    let format = get_format(path)?;
    let handle = fs::File::open(path)?;

    return Ok(File {
        path,
        handle,
        metadata,
        format
    })
}

fn validate_path(path: &Path) -> io::Result<Metadata> {
    let metadata = match path.metadata() {
        Ok(md) => md,

        // Rust appends "(os error X)" to the end of io error messages.
        // I don't like, that, so I replace them with my own messages.
        Err(err) => return Err(match err.kind() {
            ErrorKind::NotFound => Error::new(ErrorKind::NotFound, "No such file or directory"),
            ErrorKind::PermissionDenied => Error::new(ErrorKind::PermissionDenied, "Permission Denied"),

            // Should not occur.
            _ => panic!()
        })
    };

    if metadata.is_dir() {
        return Err(Error::new(ErrorKind::Other, "Is a directory"));
    }

    return Ok(metadata);
}

fn get_format(path: &Path) -> io::Result<Format> {
    if let Some(ext) = path.extension() {
        if ext == "mp3" {
            return Ok(Format::Mpeg)
        }
    }

    // Any unknown or nonexistant extensions are treated as Unknown
    return Err(Error::new(ErrorKind::Other, "Could not recognize file extension"));
}
