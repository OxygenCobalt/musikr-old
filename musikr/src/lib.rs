//! Musikr is a tag reading/writing library built primarily for the `musikr` CLI tool.

#![forbid(unsafe_code)]

#[macro_use]
mod macros;
mod raw;

pub mod err;
pub mod file;
pub mod id3v2;
pub mod string;
