//! Sending the projector picture and the room's sound to a streaming service.
//!
//! `args`, `backoff`, `redact` and `status` are plain Rust and carry the unit tests. The rest
//! drives GStreamer: `capture` takes the picture, `encode` and `encoder` make H.264 and AAC,
//! `hub` shares the result, `dest` sends it to one destination, `run` ties them together.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod args;
pub mod backoff;
pub mod capture;
pub mod dest;
pub mod encode;
pub mod encoder;
pub mod hub;
pub mod list;
pub mod redact;
pub mod report;
pub mod run;
pub mod status;
pub mod win;
