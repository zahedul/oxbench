//! Pure logic for every tool. No GUI dependency, so it stays easy to test and reuse.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod bcrypt;
pub mod epoch;
pub mod error;
pub mod hash;
pub mod id;
pub mod json;
pub mod jwt;
pub mod password;
pub mod random_string;

pub use error::{CoreError, CoreResult};
