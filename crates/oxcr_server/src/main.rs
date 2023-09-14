use bevy::prelude::App;
use oxcr_net::Plug;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_env("OXCR_LOG"))
        .init();
    let tcp = tokio::net::TcpListener::bind(("127.0.0.1", 25565)).await?;
    App::new().add_plugins(Plug::new(tcp)).run();
    Ok(())
}
