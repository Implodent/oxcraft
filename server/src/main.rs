#![feature(try_blocks, associated_type_defaults, decl_macro, iterator_try_collect)]

mod cli;
mod model;

use bevy::prelude::*;
use model::DifficultySetting;
use oxcr_protocol::{
    executor::{TaskContext, TokioTasksRuntime},
    indexmap::IndexMap,
    logging::CraftLayer,
    miette::{self, IntoDiagnostic, Report},
    model::{
        chat::{self, *},
        packets::{
            handshake::{Handshake, HandshakeNextState},
            login::{DisconnectLogin, LoginStart, LoginSuccess, SetCompression},
            play::{
                Abilities, ChangeDifficulty, DisconnectPlay, FeatureFlags, GameMode, LoginPlay,
                PlayerAbilities, PreviousGameMode, SetDefaultSpawnPosition,
            },
            status::{
                self, PingRequest, Players, PongResponse, Sample, StatusRequest, StatusResponse,
                StatusResponseJson,
            },
        },
        registry::Registry,
        DamageType, Difficulty, DimensionType, State, VarInt, WorldgenBiome, PROTOCOL_VERSION,
    },
    nbt::{nbt_serde, Nbt, NbtList, NbtTagType},
    nsfr::when_the_miette,
    rwlock_set,
    ser::{Array, Identifier, Json, Namespace, Position},
    uuid::Uuid,
    AsyncSet, PlayerN, PlayerNet, ProtocolPlugin,
};
use std::{
    net::SocketAddr,
    sync::{atomic::Ordering, Arc},
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::instrument;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::{
    error::Error,
    model::{Player, PlayerBundle, PlayerGameMode, PlayerName, PlayerUuid},
};

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

    info!(?name, ?uuid, addr=%net.peer_addr, "Player joined");

    if let Some(threshold) = net.compression {
        debug!(%threshold, ?net, "compressing");
        net.send_packet(SetCompression {
            threshold: VarInt::<i32>(threshold.try_into().unwrap()),
        })?;
        net.compressing.set(true).await;
    }

    net.send_packet(LoginSuccess {
        uuid,
        username: name.clone(),
    })?;

    net.state.set(State::Play).await;

    let game_mode = GameMode::Survival;

    let player = PlayerBundle {
        name: PlayerName(name.clone()),
        uuid: PlayerUuid(uuid),
        game_mode: PlayerGameMode(game_mode),
        player_marker: Player,
    };

    let registry_codec = cx
        .run_on_main_thread(move |w| {
            let _ = w.world.entity_mut(ent_id).insert(player);
            let dimension_types = w.world.resource::<Registry<DimensionType>>();
            let worldgen_biomes = w.world.resource::<Registry<WorldgenBiome>>();
            let damage_types = w.world.resource::<Registry<DamageType>>();

            Ok::<_, oxcr_protocol::nbt::NbtError>(IndexMap::from([
                (
                    "minecraft:dimension_type".to_string(),
                    nbt_serde(dimension_types)?,
                ),
                (
                    "minecraft:damage_type".to_string(),
                    nbt_serde(damage_types)?,
                ),
                (
                    "minecraft:worldgen/biome".to_string(),
                    nbt_serde(worldgen_biomes)?,
                ),
                (
                    "minecraft:chat_type".to_string(),
                    Nbt::Compound(IndexMap::from([
                        (
                            "type".to_string(),
                            Nbt::String("minecraft:chat_type".to_string()),
                        ),
                        (
                            "value".to_string(),
                            Nbt::ListTyped(NbtList {
                                tag: NbtTagType::Compound,
                                tags: vec![],
                            }),
                        ),
                    ])),
                ),
            ]))
        })
        .await?;

    debug!("{registry_codec:#?}");

    let login_play = LoginPlay {
        entity_id: ent_id.index() as i32,
        game_mode,
        prev_game_mode: PreviousGameMode::Undefined,
        registry_codec,
        enable_respawn_screen: true,
        is_hardcore: false,
        dimension_names: Array::new(&[Identifier::new(Namespace::Minecraft, "overworld")]),
        dimension_name: Identifier::new(Namespace::Minecraft, "overworld"),
        dimension_type: Identifier::new(Namespace::Minecraft, "overworld"),
        hashed_seed: 0,
        death_location: None,
        is_debug: false,
        is_flat: false,
        max_players: VarInt(1),
        reduced_debug_info: false,
        simulation_distance: VarInt(2),
        view_distance: VarInt(2),
        portal_cooldown: VarInt(20),
    };

    net.send_packet(login_play)?;

    let difficulty = cx
        .run_on_main_thread(move |w| *w.world.resource::<DifficultySetting>())
        .await;

    net.send_packet(FeatureFlags {
        feature_flags: Array::new(&[FeatureFlags::FEATURE_VANILLA]),
    })?;

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

    net.send_packet(SetDefaultSpawnPosition {
        location: Position {
            x: 0i16.into(),
            z: 0i16.into(),
            y: 50i8.into(),
        },
        angle: 0f32,
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
            let cancellator = CancellationToken::new();

            let CompressionThreshold(compress) = t
                .run_on_main_thread(|tcx| *tcx.world.resource::<CompressionThreshold>())
                .await;

            let player = PlayerNet::new(read, write, cancellator.child_token(), compress);
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
                cancellator.cancelled_owned().await;

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
        rt.spawn_background_task(move |task| async move {
            let cx = Arc::new(task);
            tokio::select! {
                lfc = lifecycle(player.clone(), cx.clone(), event.entity) => {
                    match when_the_miette(lfc) {
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

                            player.cancellator.cancel();

                            Ok(())
                        }
                    }
                },
                () = player.cancellator.cancelled() => {
                    error!("connection terminated");
                    Ok(())
                }
            }
        });
    }
}

#[instrument]
fn init_registries(
    mut dimension_types: ResMut<Registry<DimensionType>>,
    mut worldgen_biomes: ResMut<Registry<WorldgenBiome>>,
    mut damage_types: ResMut<Registry<DamageType>>,
) {
    info!("initializing registries...");

    dimension_types
        .0
        .extend([("minecraft:overworld".to_string(), DimensionType::OVERWORLD)]);

    worldgen_biomes
        .0
        .extend([("minecraft:plains".to_string(), WorldgenBiome::PLAINS)]);

    damage_types.0.extend([
        ("minecraft:arrow".to_string(), DamageType::ARROW),
        (
            "minecraft:bad_respawn_point".to_string(),
            DamageType::BAD_RESPAWN_POINT,
        ),
    ]);

    info!(dimension_types=?dimension_types.0, worldgen_biomes=?worldgen_biomes.0, damage_types=?damage_types.0, "successfully initialized registries.");
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct CompressionThreshold(pub Option<usize>);

#[tokio::main]
async fn main() -> Result<(), Report> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_env("OXCR_LOG"))
        .with(CraftLayer)
        .init();

    miette::set_panic_hook();

    let cli = cli::Cli::parse_args()?;

    debug!(?cli);

    trace!(
        r#"Hello, dude with OXCR_LOG=trace.
I see you have some intention of getting more info out of this program.
Please be aware that if you do not turn OXCR_LOG=trace down
to something like =debug or =info, your log files might explode.
That was my warning, now I wish you good luck debugging your issue."#
    );

    let tcp = tokio::net::TcpListener::bind(("127.0.0.1", cli.port))
        .await
        .into_diagnostic()?;

    info!("Starting server on port {}", cli.port);

    App::new()
        .add_plugins(ProtocolPlugin)
        .add_event::<PlayerLoginEvent>()
        .insert_resource(NetNet(Arc::new(Network { tcp })))
        .insert_resource(DifficultySetting {
            difficulty: Difficulty::Hard,
            is_locked: false,
        })
        .init_resource::<Registry<DimensionType>>()
        .init_resource::<Registry<WorldgenBiome>>()
        .init_resource::<Registry<DamageType>>()
        .insert_resource(CompressionThreshold(Some(256)))
        .add_systems(Startup, (init_registries, listen))
        .add_systems(Update, on_login)
        .run();

    Ok(())
}
