mod error;
pub mod model;
mod ser;

use chashmap::CHashMap;
use error::{Error, Result};
use ser::Deserialize;
use std::{
    io::{BorrowedCursor, Cursor},
    net::SocketAddr,
    ops::Generator,
    sync::Arc,
};

use apecs::*;
use model::{
    packets::{Packet, PacketClientbound, PacketServerbound, SerializedPacket},
    State, VarInt,
};
use tokio::{
    io::AsyncReadExt,
    sync::{broadcast, mpsc, RwLock},
};

pub struct ServerLock(pub Arc<RwLock<Server>>);

#[derive(Debug)]
pub struct Server {
    pub players: CHashMap<SocketAddr, PlayerNet>,
    pub tcp: tokio::net::TcpListener,
}

#[derive(Debug)]
pub struct PlayerNet {
    send: mpsc::UnboundedSender<SerializedPacket>,
    recv: broadcast::Receiver<SerializedPacket>,
    pub addr: SocketAddr,
    pub state: State,
}

impl PlayerNet {
    pub async fn recv_packet<T: Packet>(&self) -> Result<T> {
        self.recv.recv().await?;
    }
    pub fn send_packet<T: Packet>(&self, packet: T) -> Result<()> {
        Ok(self.send.send(SerializedPacket::new(packet))?)
    }
}

pub async fn accept_connections() {}
