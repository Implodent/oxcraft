#![feature(default_free_fn)]

use std::{default::default, sync::Arc};

use apecs::*;
use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("hi");
    let tcp = tokio::net::TcpListener::bind(("127.0.0.1", 25565)).await?;
    let server = oxcr_net::ServerLock(Arc::new(Mutex::new(oxcr_net::Server {
        players: default(),
        tcp,
    })));
    let mut world_builder = World::builder();
    world_builder.with_plugin(oxcr_net::OxCraftNetPlugin);
    let mut world = world_builder.build()?;
    world.with_resource(server)?;
    world.run()?;
    Ok(())
}
