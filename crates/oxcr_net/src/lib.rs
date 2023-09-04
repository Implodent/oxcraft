#![feature(associated_type_defaults)]

mod error;
mod executor;
pub mod model;
mod ser;

/// Equivalent of Zig's `unreachable` in ReleaseFast/ReleaseSmall mode
macro_rules! explode {
    () => {
        unsafe { std::hint::unreachable_unchecked() }
    };
}

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
    fmt::Debug,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;

use executor::*;
use model::{
    chat::{ChatColor, ChatComponent, ChatStringComponent},
    packets::{
        handshake::Handshake,
        login::LoginStart,
        status::{
            PingRequest, Players, PongResponse, Sample, StatusRequest, StatusResponse,
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
    select,
    sync::{mpsc, oneshot, RwLock},
};

use crate::model::packets::{
    handshake::HandshakeNextState,
    login::{DisconnectLogin, LoginSuccess},
    play::DisconnectPlay,
};

#[derive(Debug)]
pub struct PlayerNet {
    send: flume::Sender<SerializedPacket>,
    recv: flume::Receiver<SerializedPacket>,
    pub addr: SocketAddr,
    pub state: RwLock<State>,
}

#[derive(Component, Deref, Debug)]
#[deref(forward)]
pub struct PlayerN(pub Arc<PlayerNet>);

#[derive(Component, Debug)]
pub struct Player {
    pub name: FixedStr<16, YesSync>,
    pub uuid: Uuid,
}

unsafe impl Send for PlayerNet {}
unsafe impl Sync for PlayerNet {}

impl PlayerNet {
    pub fn new(
        addr: SocketAddr,
        mut read: OwnedReadHalf,
        mut write: OwnedWriteHalf,
        shit: oneshot::Sender<()>,
    ) -> Self {
        let (s_recv, recv) = flume::unbounded();
        let (send, r_send) = flume::unbounded();
        let send_task = tokio::spawn(async move {
            async {
                loop {
                    let packet: SerializedPacket = r_send.recv_async().await?;
                    let data = packet.serialize();
                    write.write_all(&data).await?;
                }
                #[allow(unreachable_code)]
                Ok::<(), crate::error::Error>(())
            }
            .await?;

            drop(write);
            Ok::<(), crate::error::Error>(())
        });
        let recv_task = tokio::spawn(async move {
            async {
                let mut buf = BytesMut::new();

                loop {
                    let read_bytes = read.read_buf(&mut buf).await?;
                    if read_bytes == 0 {
                        return Ok(());
                    }
                    let spack = SerializedPacket::deserialize
                        .parse_from(&buf.as_ref())
                        .into_result()?;
                    s_recv.send_async(spack).await?;
                    buf.clear();
                }
                #[allow(unreachable_code)]
                Ok::<(), crate::error::Error>(())
            }
            .await?;
            drop(read);

            Ok::<(), crate::error::Error>(())
        });

        tokio::spawn(async move {
            select! {
                Ok(Ok(())) = recv_task => {
                    info!(%addr, "Disconnected");
                    shit.send(()).expect("the fuck????");
                }
                Ok(Err(error)) = send_task => {
                    warn!(%addr, ?error, "Disconnected (error)");
                    shit.send(()).expect("THE FUCK????");
                }
            }
        });
        Self {
            send,
            recv,
            addr,
            state: RwLock::new(State::Handshaking),
        }
    }

    /// Reads a packet.
    pub async fn recv_packet<T: Packet + Deserialize<Context = PacketContext> + Debug>(
        &self,
    ) -> Result<T> {
        let sp = self.recv.recv_async().await?;
        let state = *self.state.read().await;
        let context = PacketContext { id: sp.id, state };

        debug!(addr=%self.addr, ?context, ?sp, "receiving packet");

        let mut input = InputOwned::from_input_with_context(sp.data.as_ref(), context);
        let result = ParseResult::single(T::deserialize(input.as_ref_at_zero())).into_result();
        debug!(?result, %self.addr, "Received packet");
        result
    }

    /// Writes a packet.
    pub fn send_packet<T: Packet + Serialize + Debug>(&self, packet: T) -> Result<()> {
        debug!(?packet, addr=%self.addr, "Sending packet");
        Ok(self.send.send(SerializedPacket::new(packet))?)
    }

    pub async fn lifecycle(&self, cx: Arc<TaskContext>, ent_id: Entity) -> Result<()> {
        let handshake: Handshake = self.recv_packet().await?;
        debug!(?handshake, "Handshake");

        match handshake.next_state {
            HandshakeNextState::Login => self.login(cx, ent_id).await,
            HandshakeNextState::Status => self.status().await,
        }
    }

    async fn login(&self, cx: Arc<TaskContext>, ent_id: Entity) -> Result<()> {
        rwlock_set(&self.state, State::Login).await;

        let LoginStart { name, uuid } = self.recv_packet().await?;
        debug!(login.name=?name, login.uuid=?uuid, %self.addr, "Login");
        let uuid = uuid.unwrap_or_else(|| {
            let real = format!("OfflinePlayer:{name}");
            Uuid::new_v3(&Uuid::NAMESPACE_DNS, real.as_bytes())
        });

        info!(?name, ?uuid, addr=?self.addr, "Player joined");

        self.send_packet(LoginSuccess {
            username: name.clone(),
            uuid,
            properties: Array::empty(),
        })?;

        let player = Player { name, uuid };

        cx.run_on_main_thread(move |w| {
            let _ = w.world.entity_mut(ent_id).insert(player);
        })
        .await;

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
                description: model::chat::ChatComponent::Multi(vec![ChatComponent::String(
                    ChatStringComponent {
                        text: "help".into(),
                        basic: model::chat::BasicChatComponent {
                            bold: true,
                            color: Some(ChatColor::Named(model::chat::ChatColorNamed::Aqua)),
                            ..Default::default()
                        },
                    },
                )]),
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
    let net = (*net).clone_arc().0;
    info!("listening");
    rt.spawn_background_task(move |_task| async move {
        let task = Arc::new(_task);
        loop {
            let t = task.clone();
            debug!("listen loop");
            let (tcp, addr) = net.listen.accept().await?;
            info!(%addr, "accepted");

            let (read, write) = tcp.into_split();
            let (shit, shit_r) = oneshot::channel();

            let player = PlayerNet::new(addr, read, write, shit);
            let entity = t
                .clone()
                .run_on_main_thread(move |cx| {
                    let entity = cx.world.spawn((PlayerN(Arc::new(player)),)).id();
                    cx.world.send_event(PlayerLoginEvent { entity, addr });
                    entity
                })
                .await;
            tokio::spawn(async move {
                let taske = t;
                shit_r.await.expect("AAAAAAAAAAAAAAAAAAAAAAAAAA");
                taske
                    .run_on_main_thread(move |cx| {
                        if !cx.world.despawn(entity) {
                            error!("the fuck");
                            explode!();
                        }
                    })
                    .await;
            });
        }
        #[allow(unreachable_code)]
        Ok::<(), Error>(())
    });
}

fn on_login(rt: Res<TokioTasksRuntime>, mut ev: EventReader<PlayerLoginEvent>, q: Query<&PlayerN>) {
    for event in ev.iter().cloned() {
        info!(%event.addr, "Logged in");
        let player = q.get(event.entity).unwrap().0.clone();
        rt.spawn_background_task(move |task| async move {
            let cx = Arc::new(task);
            match player.lifecycle(cx.clone(), event.entity).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!(error=?e, ?player, "Disconnecting");
                    match *(player.state.read().await) {
                        State::Login => player.send_packet(DisconnectLogin {
                            reason: Json(ChatComponent::String(ChatStringComponent {
                                text: format!("{e}"),
                                ..Default::default()
                            })),
                        }),
                        State::Play => player.send_packet(DisconnectPlay {
                            reason: Json(ChatComponent::String(ChatStringComponent {
                                text: format!("{e}"),
                                ..Default::default()
                            })),
                        }),
                        _ => Ok(()),
                    }
                }
            }
        });
    }
}
