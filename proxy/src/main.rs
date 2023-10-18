use oxcr_protocol::logging::CraftLayer;
use oxcr_protocol::miette::Report;
use tracing::info;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

#[tokio::main]
async fn main() -> Result<(), Report> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_env("OXCR_LOG"))
        .with(CraftLayer)
        .init();

    info!("henlo worled me is proxe");

    Ok(())
}
