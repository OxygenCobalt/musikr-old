//! Powerful audio metadata manipulation.
//!
//! Musikr is an audio metadata reading/writing library.
//!
//! TODO: Add formats later on as this project continues to develop
//!
//! Musikr was primarily built for the `musikr-cli` tool, and not wanting to rely on an
//! unsafe library like Taglib for the project, it was decided to built a pure safe-rust
//! library aiming for the following.
//!
//! - **Musikr is low-level.** Rust is not good at making deep abstraction layers like C++
//! or Python, so musikr only provides the baseline interfaces so you dont have to fiddle
//! with the bits yourself. Musikr will assume that you have a working understanding of
//! the tag format you're dealing with, but will still try to provide helpful explanations
//! in the API documentation.
//! - **Musikr is powerful.** Musikr tries to implement most if not all of a tag format,
//! even the obscure parts like extended headers or `EQU2` frames, allowing for the deep
//! manipulation of audio metadata while still retaining ergonomics.
//! - **Musikr is safe.** Musikr is written in 100% safe rust, with automatic testing
//! and fuzzing to ensure correctness when parsing files. Musikr should be able to handle
//! any file given to it, and if it doesn't, then it will be made to.

#![forbid(unsafe_code)]

#[macro_use]
mod core;

pub mod id3v2;
pub mod string;
