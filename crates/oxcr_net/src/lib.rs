#![feature(generators, generator_trait)]
mod error;
pub mod model;

use chashmap::CHashMap;
use error::{Error, Result};
use std::{net::SocketAddr, ops::Generator, sync::Arc};

use apecs::*;
use model::{
    packets::{Packet, PacketClientbound},
    State,
};
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, RwLock},
};

pub struct ServerLock(pub Arc<RwLock<Server>>);

#[derive(Debug)]
pub struct Server {
    pub players: CHashMap<SocketAddr, PlayerNet>,
    pub tcp: tokio::net::TcpListener,
}

#[derive(Debug)]
pub struct PlayerNet {
    send: mpsc::UnboundedReceiver<PacketClientbound>,

    pub addr: SocketAddr,
    pub state: State,
}

impl PlayerNet {
    pub async fn recv_packet<T: Packet>(&self) -> Result<T> {}
}

pub struct OxCraftNetPlugin;
impl Plugin for OxCraftNetPlugin {
    fn apply(self, builder: &mut WorldBuilder) {
        builder.with_async("accept_connections", accept_connections);
    }
}

pub async fn accept_connections(mut facade: Facade) -> anyhow::Result<()> {
    let server_lock = facade
        .visit(|server: Read<ServerLock, NoDefault>| server.0.clone())
        .await?;
    loop {
        let server = server_lock.read().await;
        let (sock, addr) = server.tcp.accept().await?;
        let (mut read, mut write) = sock.into_split();
        let (tx_cb, rx_cb) = mpsc::unbounded_channel();
        let (tx_sb, rx_sb) = tokio::sync::broadcast::channel(100);

        let net = PlayerNet {
            addr,
            state: State::Handshaking,
        };
        drop(server);
        let server = server_lock.write().await;

        if server.players.insert(addr, net).is_some() {
            let _ = server.players.remove(&addr);
            return Err(Error::DupePlayer.into());
        }

        facade
            .spawn(async move {
                if let Err::<(), Error>(e) = async move {
                    let mut buf = bytes::BytesMut::with_capacity(model::MAX_PACKET_DATA);
                    loop {
                        read.read_buf(&mut buf).await?;
                    }
                }
                .await
                {}
            })
            .detach();
    }
}
