#![feature(iterator_try_collect)]

use oxcr_protocol::{
    miette::{self, bail, Report},
    nsfr::when_the_miette,
};

mod cli;
mod error;

fn run() -> Result<(), Report> {
    let mut args = std::env::args();

    match args.next() {
        None => bail!("the"),
        Some(path) => {
            let arguments: String = args.collect();
            Ok(())
        }
    }
}

fn main() {
    match when_the_miette(run()) {
        Ok(()) => {}
        Err(_) => {}
    }
}
