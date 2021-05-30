//! libmusikr is the tagging library that is used by the main CLI application.
//! currently this library is only intended for the musikr application.

#![forbid(unsafe_code)]
#![allow(dead_code)] // Temporary until all parts of the lib are fleshed out

pub mod file;
pub mod id3;
