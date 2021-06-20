//! libmusikr is the tagging library used by the musikr CLI program.
//! As of now, it is not meant to wider use.

#![forbid(unsafe_code)]

#[macro_use]
mod macros;

pub mod err;
pub mod file;
pub mod id3v2;

mod raw;
mod string;
