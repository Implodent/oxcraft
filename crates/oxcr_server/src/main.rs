use apecs::*;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("hi");
    let mut world = World::builder()
        .with_async("server_lifecycle", oxcr_net::lifecycle)
        .build()?;
    world.run()?;
    Ok(())
}
