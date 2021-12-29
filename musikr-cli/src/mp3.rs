mod id3v2;

use crate::print_header;
use crate::show::{ShowError, ShowResult, TagFilter};
use log::error;
use std::path::Path;

pub fn show<'a>(path: &Path, filter: TagFilter) -> ShowResult {
    // MP3 contains 3 major metadata formats.
    // Try them all and see if any of them give any output.
    let id3v2_tags = match musikr::id3v2::Tag::open(path) {
        Ok(tag) => id3v2::show(tag, filter),
        Err(err) => {
            error!("failed to parse id3v2 tag: {}", err);
            Vec::new()
        }
    };

    if !id3v2_tags.is_empty() {
        print_header!("Metadata for {}:", path.display());
        println!("  ID3v2:");

        for tag in id3v2_tags {
            tag.print(4)
        }

        Ok(())
    } else {
        Err(ShowError::NoMetadata)
    }
}
