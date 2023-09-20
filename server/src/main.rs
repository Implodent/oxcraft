#![feature(try_blocks, associated_type_defaults, decl_macro, iterator_try_collect)]

mod model;

use bevy::prelude::*;
use model::DifficultySetting;
use oxcr_protocol::{
    bytes::Bytes,
    executor::{TaskContext, TokioTasksRuntime},
    model::{
        chat::{self, *},
        packets::{
            handshake::{Handshake, HandshakeNextState},
            login::{DisconnectLogin, LoginStart, LoginSuccess},
            play::{
                Abilities, ChangeDifficulty, DisconnectPlay, GameMode, LoginPlay, PlayerAbilities,
                PreviousGameMode,
            },
            status::{
                self, PingRequest, Players, PongResponse, Sample, StatusRequest, StatusResponse,
                StatusResponseJson,
            },
        },
        Difficulty, State, VarInt, PROTOCOL_VERSION,
    },
    nbt::NbtJson,
    rwlock_set,
    ser::{Array, Identifier, Json, Namespace},
    serde::json,
    uuid::Uuid,
    AsyncSet, PlayerN, PlayerNet, ProtocolPlugin,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};
use tracing_subscriber::EnvFilter;

use crate::{error::Error, model::Player};

mod error;

type Result<T, E = error::Error> = ::std::result::Result<T, E>;

async fn lifecycle(net: Arc<PlayerNet>, cx: Arc<TaskContext>, ent_id: Entity) -> Result<()> {
    let handshake: Handshake = net.recv_packet().await?;
    debug!(?handshake, "Handshake");

    if handshake.protocol_version.0 != PROTOCOL_VERSION {
        return Err(Error::IncorrectVersion(handshake.protocol_version.0));
    }

    match handshake.next_state {
        HandshakeNextState::Login => login(net, cx, ent_id).await,
        HandshakeNextState::Status => status(net).await,
    }
}

async fn login(net: Arc<PlayerNet>, cx: Arc<TaskContext>, ent_id: Entity) -> Result<()> {
    net.state.set(State::Login).await;

    let LoginStart { name, uuid } = net.recv_packet().await?;

    debug!(login.name=?name, login.uuid=?uuid, %net.peer_addr, "Login Start");

    let uuid = uuid.unwrap_or_else(|| {
        debug!(?name, "Player is in offline mode");
        let real = format!("OfflinePlayer:{name}");
        Uuid::new_v3(&Uuid::NAMESPACE_DNS, real.as_bytes())
    });

    let uuid_bytes = Bytes::copy_from_slice(uuid.as_bytes());

    info!(?name, ?uuid, ?uuid_bytes, addr=%net.peer_addr, "Player joined");

    net.send_packet(LoginSuccess {
        uuid,
        username: name.clone(),
    })?;

    net.state.set(State::Play).await;

    let game_mode = GameMode::Survival;

    let player = Player {
        name: name.clone(),
        uuid,
        game_mode,
    };

    cx.run_on_main_thread(move |w| {
        let _ = w.world.entity_mut(ent_id).insert(player);
    })
    .await;

    net.send_packet(LoginPlay {
        entity_id: ent_id.index() as i32,
        game_mode,
        prev_game_mode: PreviousGameMode::Undefined,
        registry_codec: NbtJson(json::json!({
            "minecraft:dimension_type": {
                "type": "minecraft:dimension_type",
                "value": [
                    {
                        "name": "minecraft:overworld",
                        "id": 0,
                        "element": {
                            "piglin_safe": false,
                            "has_raids": true,
                            "monster_spawn_light_level": 1,
                            "monster_spawn_block_light_limit": 0,
                            "natural": true,
                            "ambient_light": 0.0,
                            "infiniburn": "#minecraft:infiniburn_overworld",
                            "respawn_anchor_works": false,
                            "has_skylight": true,
                            "bed_works": true,
                            "effects": "minecraft:overworld",
                            "min_y": -64,
                            "height": 320,
                            "logical_height": 300,
                            "coordinate_scale": 1.0,
                            "ultrawarm": false,
                            "has_ceiling": false
                        }
                    }
                ]
            },
            "minecraft:worldgen/biome": {
                "type": "minecraft:worldgen/biome",
                "value": [
                    {
                        "name": "minecraft:plains",
                        "id": 0,
                        "element": {
                            "has_precipitation": false,
                            "depth": 0.7,
                            "temperature": 0.7,
                            "scale": 1.0,
                            "downfall": 1.0,
                            "category": "plains",
                            "temperature_modifier": "frozen",
                            "effects": {
                                "sky_color": 0x7fa1ff,
                                "water_fog_color": 0x7fa1ff,
                                "fog_color": 0x7fa1ff,
                                "water_color": 0x8fa1ff,
                                "foliage_color": 0xffa1ff,
                                "grass_color": 0x0a1fb5
                            }
                        }
                    }
                ]
            },
            "minecraft:chat_type": {
                "type": "minecraft:chat_type",
                "value": [
                    {
                        "name": "minecraft:chat",
                        "id": 0,
                        "elements": {
                            "chat": {
                                "translation_key": "chat.type.text",
                                "parameters": ["sender", "content"]
                            },
                            "narration": {
                                "translation_key": "chat.type.text.narrate",
                                "parameters": ["sender", "content"]
                            }
                        }
                    }
                ]
            }
        })),
        enable_respawn_screen: true,
        is_hardcore: false,
        dimension_names: Array::new(&[Identifier::new(Namespace::Minecraft, "overworld")]),
        dimension_name: Identifier::new(Namespace::Minecraft, "overworld"),
        dimension_type: Identifier::new(Namespace::Minecraft, "overworld"),
        hashed_seed: 0xfaf019,
        death_location: None,
        is_debug: false,
        is_flat: false,
        max_players: VarInt(0x1000),
        reduced_debug_info: false,
        portal_cooldown: VarInt(0),
        simulation_distance: VarInt(8),
        view_distance: VarInt(8),
    })?;

    let difficulty = cx
        .run_on_main_thread(move |w| *w.world.resource::<DifficultySetting>())
        .await;

    net.plugin_message(Identifier::MINECRAFT_BRAND, "implodent")?;

    net.send_packet(ChangeDifficulty {
        difficulty: difficulty.difficulty,
        difficulty_locked: difficulty.is_locked,
    })?;

    net.send_packet(PlayerAbilities {
        flags: Abilities::FLYING,
        flying_speed: 0.05f32,
        fov_modifier: 0.1f32,
    })?;

    Ok(())
}

async fn status(net: Arc<PlayerNet>) -> Result<()> {
    rwlock_set(&net.state, State::Status).await;

    let _: StatusRequest = net.recv_packet().await?;
    net.send_packet(StatusResponse {
        json_response: Json(StatusResponseJson {
            enforces_secure_chat: false,
            version: status::Version {
                name: String::from("Implodent"),
                protocol: PROTOCOL_VERSION,
            },
            description: chat::ChatComponent::Multi(vec![
                ChatComponent::String(ChatStringComponent {
                    text: "help\n".into(),
                    basic: chat::BasicChatComponent {
                        bold: true,
                        color: Some(ChatColor::Named(chat::ChatColorNamed::Aqua)),
                        ..Default::default()
                    },
                }),
                ChatComponent::String(ChatStringComponent {
                    text: "help please".into(),
                    basic: BasicChatComponent {
                        italic: true,
                        color: Some(ChatColor::Named(chat::ChatColorNamed::Gold)),
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

    let PingRequest { payload } = net.recv_packet().await?;
    net.send_packet(PongResponse { payload })?;

    Ok(())
}

pub struct Network {
    pub tcp: TcpListener,
}

/// Do not, EVER, acquire a [`ResMut<NetNet>`]. Everything will explode.
#[derive(Resource)]
struct NetNet(pub Arc<Network>);
impl Clone for NetNet {
    fn clone(&self) -> Self {
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
    let net = (net.into_inner()).0.clone();

    info!("listening");

    rt.spawn_background_task(move |_task| async move {
        let task = Arc::new(_task);
        loop {
            let t = task.clone();
            let (tcp, addr) = net.tcp.accept().await?;
            info!(%addr, "accepted");

            let (read, write) = tcp.into_split();
            let (shit, mut shit_r) = mpsc::channel(1);

            let player = PlayerNet::new(read, write, shit.clone());
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
                drop(shit_r);
                taske
                    .run_on_main_thread(move |cx| {
                        if !cx.world.despawn(entity) {
                            error!(?entity, "despawn failed");
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
            match lifecycle(player.clone(), cx.clone(), event.entity).await {
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
                    shit.send(())
                        .await
                        .unwrap_or_else(|_| error!("disconnect failed (already disconnected)"));
                    Ok(())
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_env("OXCR_LOG"))
        .init();
    let tcp = tokio::net::TcpListener::bind(("127.0.0.1", 25565)).await?;
    App::new()
        .add_plugins(ProtocolPlugin)
        .add_event::<PlayerLoginEvent>()
        .insert_resource(NetNet(Arc::new(Network { tcp })))
        .insert_resource(DifficultySetting {
            difficulty: Difficulty::Hard,
            is_locked: false,
        })
        .add_systems(Startup, listen)
        .add_systems(Update, on_login)
        .run();
    Ok(())
}
