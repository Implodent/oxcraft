#![feature(associated_type_defaults)]
#![feature(iterator_try_collect)]
#![feature(decl_macro)]
#![feature(try_blocks)]

mod error;
mod executor;
pub mod model;
pub mod nbt;
pub mod nsfr;
mod ser;

/// Equivalent of Zig's `unreachable` in ReleaseFast/ReleaseSmall mode
#[macro_export]
macro_rules! explode {
    () => {{
        #[cfg(not(debug_assertions))]
        unsafe {
            std::hint::unreachable_unchecked()
        }
        #[cfg(debug_assertions)]
        {
            unreachable!()
        }
    }};
}

async fn rwlock_set<T>(rwlock: &RwLock<T>, value: T) {
    let mut w = rwlock.write().await;
    *w = value;
}

use aott::prelude::Parser;
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
    chat::{BasicChatComponent, ChatColor, ChatComponent, ChatStringComponent},
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
    sync::{mpsc, RwLock},
};

use crate::{
    model::{
        packets::{
            handshake::HandshakeNextState,
            login::{DisconnectLogin, LoginSuccess},
            play::{DisconnectPlay, LoginPlay},
        },
        player::Player, VarInt,
    },
    nbt::NbtJson,
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

unsafe impl Send for PlayerNet {}
unsafe impl Sync for PlayerNet {}

impl PlayerNet {
    pub fn new(
        addr: SocketAddr,
        mut read: OwnedReadHalf,
        mut write: OwnedWriteHalf,
        shit: mpsc::Sender<()>,
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
                    let spack = SerializedPacket::deserialize.parse(buf.as_ref())?;
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
                    shit.send(()).await.expect("the fuck????");
                }
                Ok(Err(error)) = send_task => {
                    warn!(%addr, ?error, "Disconnected (error)");
                    shit.send(()).await.expect("THE FUCK????");
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

        let result = T::deserialize.parse_with_context(sp.data.as_ref(), context);
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

        let game_mode = model::player::GameMode::Survival;

        let player = Player {
            name: name.clone(),
            uuid,
            game_mode,
        };

        cx.run_on_main_thread(move |w| {
            let _ = w.world.entity_mut(ent_id).insert(player);
        })
        .await;

        self.send_packet(LoginPlay {
            entity_id: ent_id.index() as i32,
            game_mode,
            prev_game_mode: model::player::PreviousGameMode::Undefined,
            registry_codec: NbtJson(serde_json::from_str(model::packets::play::json::CODEC_120)?),
            enable_respawn_screen: true,
            is_hardcore: false,
            dimension_names: Array::new(&[Identifier::new(Namespace::Minecraft, "overworld")]),
            dimension_name: Identifier::new(Namespace::Minecraft, "overworld"),
            dimension_type: Identifier::new(Namespace::Minecraft, "the-what"),
            hashed_seed: 0xfaf019,
            death_location: None,
            is_debug: false,
            is_flat: false,
            max_players: VarInt(0x1000),
            reduced_debug_info: false,
            portal_cooldown: VarInt(0),
            simulation_distance: VarInt(8),
            view_distance: VarInt(8),
        });

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
                description: model::chat::ChatComponent::Multi(vec![
                    ChatComponent::String(ChatStringComponent {
                        text: "help\n".into(),
                        basic: model::chat::BasicChatComponent {
                            bold: true,
                            color: Some(ChatColor::Named(model::chat::ChatColorNamed::Aqua)),
                            ..Default::default()
                        },
                    }),
                    ChatComponent::String(ChatStringComponent {
                        text: "help please".into(),
                        basic: BasicChatComponent {
                            italic: true,
                            color: Some(ChatColor::Named(model::chat::ChatColorNamed::Gold)),
                            ..Default::default()
                        },
                    }),
                ]),
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
    pub shit: mpsc::Sender<()>,
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
            let (shit, mut shit_r) = mpsc::channel(1);

            let player = PlayerNet::new(addr, read, write, shit.clone());
            let entity = t
                .clone()
                .run_on_main_thread(move |cx| {
                    let entity = cx.world.spawn((PlayerN(Arc::new(player)),)).id();
                    cx.world.send_event(PlayerLoginEvent { entity, addr, shit });
                    entity
                })
                .await;
            tokio::spawn(async move {
                let taske = t;
                shit_r.recv().await.expect("AAAAAAAAAAAAAAAAAAAAAAAAAA");
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
        let shit = event.shit.to_owned();
        rt.spawn_background_task(move |task| async move {
            let cx = Arc::new(task);
            match player.lifecycle(cx.clone(), event.entity).await {
                Ok(()) => Ok::<(), Error>(()),
                Err(e) => {
                    error!(error=?e, ?player, "Disconnecting");

                    // ignore the result because we term the connection afterwards
                    let _ = match *(player.state.read().await) {
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
                    };
                    shit.send(()).await.expect("the f u c k ???");
                    Ok(())
                }
            }
        });
    }
}
