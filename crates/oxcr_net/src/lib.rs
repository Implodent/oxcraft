#![feature(associated_type_defaults)]

mod error;
mod executor;
pub mod model;
mod ser;

async fn rwlock_set<T>(rwlock: &RwLock<T>, value: T) {
    let mut w = rwlock.write().await;
    *w = value;
}

use aott::{
    error::ParseResult,
    prelude::{InputOwned, Parser},
};
use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, time::TimePlugin};
use bytes::BytesMut;
use error::{Error, Result};
use ser::*;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use executor::*;
use model::{
    packets::{
        handshake::Handshake,
        status::{
            Description, PingRequest, Players, PongResponse, Sample, StatusRequest, StatusResponse,
            StatusResponseJson, Version,
        },
        Packet, PacketContext, SerializedPacket,
    },
    State,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
    sync::RwLock,
    task::JoinHandle,
};

use crate::model::packets::handshake::HandshakeNextState;

#[derive(Debug)]
pub struct PlayerNet {
    send: flume::Sender<SerializedPacket>,
    recv: flume::Receiver<SerializedPacket>,
    pub addr: SocketAddr,
    pub state: RwLock<State>,
    send_task: Option<JoinHandle<Result<()>>>,
    recv_task: Option<JoinHandle<Result<()>>>,
}

#[derive(Component, Deref, Debug)]
#[deref(forward)]
pub struct PlayerN(pub Arc<PlayerNet>);

unsafe impl Send for PlayerNet {}
unsafe impl Sync for PlayerNet {}

impl PlayerNet {
    pub fn new(addr: SocketAddr, mut read: OwnedReadHalf, mut write: OwnedWriteHalf) -> Self {
        let (s_recv, recv) = flume::unbounded();
        let (send, r_send) = flume::unbounded();
        Self {
            send,
            recv,
            addr,
            state: RwLock::new(State::Handshaking),
            send_task: Some(tokio::spawn(async move {
                loop {
                    let packet = r_send.recv_async().await?;
                    let data = packet.serialize();
                    write.write_all(&data).await?;
                }
            })),
            recv_task: Some(tokio::spawn(async move {
                let mut buf = BytesMut::new();
                loop {
                    let _ = read.read_buf(&mut buf).await?;
                    let spack = SerializedPacket::deserialize
                        .parse_from(&buf.as_ref())
                        .into_result()?;
                    s_recv.send_async(spack).await?;
                }
            })),
        }
    }

    /// Reads a packet.
    pub async fn recv_packet<T: Packet + Deserialize<Context = PacketContext>>(&self) -> Result<T> {
        let sp = self.recv.recv_async().await?;
        let state = *self.state.read().await;
        let context = PacketContext { id: sp.id, state };

        let mut input = InputOwned::from_input_with_context(sp.data.as_ref(), context);
        ParseResult::single(T::deserialize(input.as_ref_at_zero())).into_result()
    }

    /// Writes a packet.
    pub fn send_packet<T: Packet + Serialize>(&self, packet: T) -> Result<()> {
        Ok(self.send.send(SerializedPacket::new(packet))?)
    }

    pub async fn lifecycle(&self) -> Result<()> {
        let handshake: Handshake = self.recv_packet().await?;
        debug!(?handshake, "Handshake");

        match handshake.next_state {
            HandshakeNextState::Login => self.login().await,
            HandshakeNextState::Status => self.status().await,
        }
    }

    async fn login(&self) -> Result<()> {
        rwlock_set(&self.state, State::Login).await;

        Ok(())
    }

    async fn status(&self) -> Result<()> {
        rwlock_set(&self.state, State::Status).await;

        let _: StatusRequest = self.recv_packet().await?;
        self.send_packet(StatusResponse {
            json_response: Json(StatusResponseJson {
                enforces_secure_chat: false,
                version: Version {
                    name: String::from("Implodent"),
                    protocol: model::PROTOCOL_VERSION,
                },
                description: Description {
                    text: String::from("Me when oxcraft"),
                },
                players: Players {
                    max: -1,
                    online: 747106,
                    sample: vec![Sample {
                        id: "66b7b182-6a07-4f27-a726-69c93a06ce84".into(),
                        name: "NothinGG_".into(),
                    }],
                },
                ..Default::default()
            }),
        })?;

        let PingRequest { payload } = self.recv_packet().await?;
        self.send_packet(PongResponse { payload })?;

        Ok(())
    }
}

#[derive(Resource)]
pub struct Network {
    pub listen: TcpListener,
}

pub struct Plug {
    tcp: Mutex<Option<TcpListener>>,
}

impl Plugin for Plug {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TokioTasksPlugin::default(),
            TypeRegistrationPlugin,
            TimePlugin,
            ScheduleRunnerPlugin::run_loop(Duration::from_millis(50)),
            LogPlugin::default(),
        ))
        .add_event::<PlayerLoginEvent>()
        .insert_resource(NetNet(Arc::new(Network {
            listen: self.tcp.lock().unwrap().take().unwrap(),
        })))
        .add_systems(Startup, listen)
        .add_systems(Update, on_login);
    }
}

impl Plug {
    pub fn new(tcp: TcpListener) -> Self {
        Self {
            tcp: Mutex::new(Some(tcp)),
        }
    }
}

/// Do not, EVER, acquire a [`ResMut<NetNet>`]. Everything will explode.
#[derive(Resource)]
struct NetNet(pub Arc<Network>);
impl NetNet {
    pub fn clone_arc(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Event, Clone)]
struct PlayerLoginEvent {
    pub addr: SocketAddr,
    pub entity: Entity,
}

fn listen(net: Res<NetNet>, rt: Res<TokioTasksRuntime>) {
    let rt = rt.into_inner();
    let net = (&*net).clone_arc().0;
    info!("listening");
    rt.spawn_background_task(move |mut task| async move {
        loop {
            debug!("listen loop");
            let (tcp, addr) = net.listen.accept().await?;
            info!(%addr, "accepted");

            let (read, write) = tcp.into_split();

            let player = PlayerNet::new(addr, read, write);
            task.run_on_main_thread(move |cx| {
                let entity = cx.world.spawn((PlayerN(Arc::new(player)),)).id();
                cx.world.send_event(PlayerLoginEvent { entity, addr });
            })
            .await;
        }
        #[allow(unreachable_code)]
        Ok::<(), Error>(())
    });
}

fn on_login(rt: Res<TokioTasksRuntime>, mut ev: EventReader<PlayerLoginEvent>, q: Query<&PlayerN>) {
    for event in ev.iter().cloned() {
        info!(%event.addr, "Logged in");
        let player = q.get(event.entity).unwrap().0.clone();
        rt.spawn_background_task(move |_| async move { player.lifecycle().await });
    }
}
