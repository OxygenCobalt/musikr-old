//! Powerful audio metadata manipulation.
//!
//! Musikr is an audio metadata reading/writing library.
//!
//! TODO: Add formats later on as this project continues to develop

#[macro_use]
mod core;

pub mod id3v2;
pub use crate::core::io;
pub use crate::core::string;