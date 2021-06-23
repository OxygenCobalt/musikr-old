//! Musikr is a tag reading/writing library built primarily for the `musikr` CLI tool.

#![forbid(unsafe_code)]

#[macro_use]
mod core;

pub mod id3v2;
pub mod string;
