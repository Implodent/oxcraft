use bevy::prelude::App;
use oxcr_net::Plug;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tcp = tokio::net::TcpListener::bind(("127.0.0.1", 25565)).await?;
    App::new().add_plugins(Plug::new(tcp)).run();
    Ok(())
}
