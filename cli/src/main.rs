use oxcr_protocol::miette::Report;

mod cli;
mod error;

fn run() -> Result<(), Report> {
    let mut args = std::env::args();

    match args.next() {
        None => panic!("what"),
        Some(path) => {
            let arguments: String = args.collect();
        }
    }
}
