#![feature(iterator_try_collect, try_blocks)]

#[allow(dead_code, unused_imports)]
mod cli;
pub use cli::{flag_list, Extra, FlagName};
mod error;
pub use error::*;
