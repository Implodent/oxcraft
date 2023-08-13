mod error;
pub mod model;

use bytes::BytesMut;
use chashmap::CHashMap;
use error::Error;
use std::{net::SocketAddr, sync::Arc};

use apecs::*;
use model::packets::{PacketClientbound, PacketServerbound};
use tokio::{
    io::AsyncReadExt,
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        Mutex, Notify,
    },
};

pub struct ServerLock(pub Arc<Mutex<Server>>);

#[derive(Debug)]
pub struct Server {
    pub players: CHashMap<SocketAddr, PlayerNet>,
    pub tcp: tokio::net::TcpListener,
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

pub struct OxCraftNetPlugin;
impl Plugin for OxCraftNetPlugin {
    fn apply(self, builder: &mut WorldBuilder) {
        builder.with_async("accept_connections", accept_connections);
    }
}

pub async fn accept_connections(mut facade: Facade) -> anyhow::Result<()> {
    loop {}
}
