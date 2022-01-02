//! Audio frames.
//!
//! This module encompasses frames that aid in the decoding of media files.
//! The specification for such frames is complicated and version-specific,
//! so this module encompasses shared frames, while the sub-modules handle
//! specific frames that were revised in ID3v2.4.

pub mod v23;
pub mod v24;