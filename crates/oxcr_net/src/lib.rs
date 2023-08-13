pub mod model;

use chashmap::CHashMap;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

use apecs::*;
use model::packets::{PacketClientbound, PacketServerbound};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

#[derive(Default, Clone)]
pub struct Server(Arc<Mutex<ServerInner>>);

#[derive(Default)]
pub struct ServerInner {
    pub players: CHashMap<SocketAddr, PlayerNet>,
}

#[derive(Debug)]
pub struct PlayerNet {
    pub packets: Packets,
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub struct Packets {
    pub send: UnboundedSender<PacketClientbound>,
    pub recv: UnboundedReceiver<PacketServerbound>,
}

pub async fn lifecycle(_facade: Facade) -> anyhow::Result<()> {
    info!("hi");
    Ok(())
}
