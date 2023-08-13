mod error;
pub mod model;

use bytes::BytesMut;
use chashmap::CHashMap;
use error::Error;
use std::net::SocketAddr;

use apecs::*;
use model::packets::{PacketClientbound, PacketServerbound};
use tokio::{
    io::AsyncReadExt,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

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
    loop {
        let server = facade
            .visit(Ok::<Read<Server, NoDefault>, anyhow::Error>)
            .await?;
        let (sock, addr) = server.tcp.accept().await?;
        let (mut read, mut write) = sock.into_split();
        drop(server);
        let (tx_cb, rx_cb) = mpsc::unbounded_channel();
        let (tx_sb, rx_sb) = mpsc::unbounded_channel();
        let net = PlayerNet {
            addr,
            packets: Packets {
                send: tx_cb,
                recv: rx_sb,
            },
        };
        let server = facade
            .visit(Ok::<Write<Server, NoDefault>, anyhow::Error>)
            .await?;
        if server.players.insert(net.addr, net).is_some() {
            Err(Error::DupePlayer)?;
        }

        drop(server);
        // facade.spawn(async move {
        //     let mut buf = BytesMut::with_capacity(model::MAX_PACKET_DATA);
        //     read.read_buf(&mut buf).await.unwrap();
        // });
    }
}
