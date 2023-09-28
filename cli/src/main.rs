#![feature(iterator_try_collect, try_blocks)]

use oxcr_protocol::{
    aott::prelude::Parser,
    error::Error,
    miette::{self, bail, IntoDiagnostic, Report},
    model::packets::{play::LoginPlay, Packet, PacketContext, SerializedPacket},
    ser::{BytesSource, Deserialize, Serialize, WithSource},
    tracing::debug,
};
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

use crate::cli::{ByteInput, Cli, CliCommand};

mod cli;
mod error;

fn run(path: String, args: &str) -> Result<(), Report> {
    let tsub_guard = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_env("OXCR_LOG"))
        .finish()
        .set_default();

    let cli = cli::yay
        .parse(args)
        .map_err(|parse_error| Report::new(parse_error).with_source_code(args.to_owned()))?;

    drop(tsub_guard);

    let tsub_guard = tracing_subscriber::fmt()
        .with_env_filter(cli.level.to_string())
        .pretty()
        .set_default();

    debug!("{:?}", cli);

    match cli.command {
        CliCommand::Help => help(),
        CliCommand::Decode(inp) => {
            let bytes = match inp {
                ByteInput::Data(data) => data,
                ByteInput::File(file) => {
                    std::fs::read(std::env::current_dir().into_diagnostic()?.join(file))
                        .into_diagnostic()?
                        .into()
                }
            };
            let spack = SerializedPacket {
                length: LoginPlay::ID.length_of() + bytes.len(),
                data: bytes,
                id: LoginPlay::ID,
            };
            let deserialized: LoginPlay = spack.try_deserialize(LoginPlay::STATE)?;

            debug!("{:#?}", deserialized);
        }
    }

    Ok(())
}

fn main() -> Result<(), Report> {
    let mut args = std::env::args();

    match args.next() {
        None => bail!("the"),
        Some(path) => {
            dbg!(&path);
            let args = args.collect::<Vec<String>>().join(" ");
            run(path, &args)?;

            Ok(())
        }
    }
}

fn help() {}
