//! libmusikr is the tagging library used by the musikr CLI program.
//! As of now, it is not meant to wider use.

#![forbid(unsafe_code)]
#![allow(dead_code)] // Temporary

pub mod file;
pub mod id3;
mod raw;
