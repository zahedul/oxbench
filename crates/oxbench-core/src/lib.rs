//! Pure logic for every tool. No GUI dependency, so it stays easy to test and reuse.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod epoch;
pub mod error;
pub mod json;
pub mod jwt;

pub use error::{CoreError, CoreResult};
