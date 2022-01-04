//! Core utilities.

#[macro_use]
pub(crate) mod macros;
pub(crate) mod io;
pub(crate) mod string;

pub use {io::{BufStream, StreamError}, string::Encoding};