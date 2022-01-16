mod id3v2;

use crate::{print_header, errorln};
use crate::show::TagFilter;
use std::path::Path;

pub fn show(path: &Path, filter: TagFilter) {
    // MP3 contains 3 major metadata formats.
    // Try them all and see if any of them give any output.
    let id3v2_tags = match musikr::id3v2::Tag::open(path) {
        Ok(tag) => id3v2::show(tag, filter),
        Err(err) => {
            errorln!("{}: unable to parse id3v2 tag: {}", path.display(), err);
            Vec::new()
        }
    };

    if !id3v2_tags.is_empty() {
        print_header!("Metadata for {}:", path.display());
        println!("  ID3v2:");

        for tag in id3v2_tags {
            tag.print(4)
        }
    }
}
