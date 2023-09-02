#![feature(maybe_uninit_slice)]
#![feature(associated_type_defaults)]
mod error;
pub mod model;
mod nsfr;
mod ser;

use aott::{
    error::ParseResult,
    prelude::{InputOwned, Parser},
};
use bevy::{prelude::*, utils::hashbrown::HashMap};
use chashmap::CHashMap;
use error::{Error, Result};
use nsfr::CellerCell;
use ser::*;
use std::{
    cell::Cell,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use bevy_tokio_tasks::*;
use model::{
    packets::{Packet, PacketClientbound, PacketContext, PacketServerbound, SerializedPacket},
    State, VarInt,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
    sync::{broadcast, mpsc, RwLock},
    task::JoinHandle,
};

pub struct ServerLock(pub Arc<RwLock<Server>>);

#[derive(Debug)]
pub struct Server {
    pub players: CHashMap<SocketAddr, PlayerNet>,
    pub tcp: tokio::net::TcpListener,
}

#[derive(Debug, Component)]
pub struct PlayerNet {
    send: mpsc::UnboundedSender<SerializedPacket>,
    recv: broadcast::Receiver<SerializedPacket>,
    pub addr: SocketAddr,
    pub state: State,
    send_task: Option<JoinHandle<Result<()>>>,
    recv_task: Option<JoinHandle<Result<()>>>,
}

impl PlayerNet {
    pub fn new(
        addr: SocketAddr,
        read: OwnedReadHalf,
        write: OwnedWriteHalf,
        rt: &TokioTasksRuntime,
    ) -> Self {
        let (s_recv, recv) = broadcast::channel(100);
        let (send, r_send) = mpsc::unbounded_channel();
        Self {
            send,
            recv,
            addr,
            state: State::Handshaking,
            send_task: Some(rt.spawn_background_task(move |_| async move {
                loop {
                    let packet = r_send.recv().await.ok_or(Error::DupePlayer)?;
                    let data = packet.serialize();
                    write.write_all(&data).await?;
                }
            })),
            recv_task: Some(rt.spawn_background_task(move |_| async move {
                let mut buf = [0u8; model::MAX_PACKET_DATA];
                loop {
                    let _ = read.read(&mut buf[..]).await?;
                    let spack = SerializedPacket::deserialize
                        .parse_from(&&buf[..])
                        .into_result()?;
                    let _ = s_recv.send(spack)?;
                }
            })),
        }
    }

    /// Reads a packet.
    pub async fn recv_packet<T: Packet + Deserialize<Context = PacketContext>>(
        &mut self,
    ) -> Result<T> {
        let sp = self.recv.recv().await?;
        let state = self.state;
        let context = PacketContext { id: sp.id, state };

        let mut input = InputOwned::from_input_with_context(sp.data.as_ref(), context);
        ParseResult::single(T::deserialize(input.as_ref_at_zero())).into_result()
    }

    /// Writes a packet.
    pub fn send_packet<T: Packet + Serialize>(&self, packet: T) -> Result<()> {
        Ok(self.send.send(SerializedPacket::new(packet))?)
    }
}

#[derive(Resource)]
pub struct Network {
    pub listen: TcpListener,
    pub players: RwLock<HashMap<SocketAddr, PlayerNet>>,
}

pub struct Plug {
    tcp: Mutex<Option<TcpListener>>,
}

impl Plugin for Plug {
    fn build(&self, app: &mut App) {
        app.add_plugins(TokioTasksPlugin::default())
            .insert_resource(Network {
                listen: self.tcp.lock().unwrap().take().unwrap(),
                players: RwLock::new(HashMap::new()),
            })
            .add_systems(Update, listen);
    }
}

impl Plug {
    pub fn new(tcp: TcpListener) -> Self {
        Self {
            tcp: Mutex::new(Some(tcp)),
        }
    }
}

#[derive(Resource)]
struct NetNet(pub Arc<Network>);
impl NetNet {
    pub fn clone_arc(&self) -> Self {
        Self(self.0.clone())
    }
}

fn listen(net: Res<NetNet>, rt: Res<TokioTasksRuntime>) {
    let net = (&*net).clone_arc().0;
    rt.spawn_background_task(|task| async move {
        loop {
            let (tcp, addr) = net.listen.accept().await?;
            let (read, write) = tcp.into_split();

            let player = PlayerNet::new(addr, read, write, &*rt);
            task.run_on_main_thread(|cx| cx.world.spawn((player,)));
        }
        #[allow(unreachable_code)]
        Ok::<(), Error>(())
    });
}
