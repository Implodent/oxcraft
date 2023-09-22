use std::ops::RangeInclusive;

use bevy::prelude::{Bundle, Component, Resource};
use oxcr_protocol::{
    model::{packets::play::GameMode, Difficulty},
    ser::*,
    serde,
    uuid::Uuid,
};

use self::registry::{RegistryItem, Registry};

pub mod registry;

#[derive(Component, Debug)]
pub struct Player;

#[derive(Component, Debug)]
pub struct PlayerName(pub FixedStr<16, YesSync>);

#[derive(Component, Debug)]
pub struct PlayerUuid(pub Uuid);

#[derive(Component, Debug)]
pub struct PlayerGameMode(pub GameMode);

#[derive(Bundle, Debug)]
pub struct PlayerBundle {
    pub player_marker: Player,
    pub name: PlayerName,
    pub uuid: PlayerUuid,
    pub game_mode: PlayerGameMode,
}

#[derive(Debug, Resource, Default, Clone, Copy)]
pub struct DifficultySetting {
    pub difficulty: Difficulty,
    pub is_locked: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct DimensionType {
    pub piglin_safe: bool,
    pub has_raids: bool,
    pub monster_spawn_light_level: MonsterSpawnLightLevel,
    pub monster_spawn_block_light_limit: u8,
    pub natural: bool,
    pub ambient_light: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_time: Option<i64>,
    pub infiniburn: &'static str,
    pub respawn_anchor_works: bool,
    pub has_skylight: bool,
    pub bed_works: bool,
    pub effects: &'static str,
    pub min_y: i32,
    pub height: i32,
    pub logical_height: i32,
    pub coodrinate_scale: f64,
    pub ultrawarm: bool,
    pub has_ceiling: bool,
}

impl DimensionType {
    pub const OVERWORLD: Self = Self {
        piglin_safe: true,
        has_raids: true,
        monster_spawn_light_level: MonsterSpawnLightLevel::Range(0..=6),
        monster_spawn_block_light_limit: 0,
        natural: true,
        ambient_light: 0f32,
        fixed_time: None,
        infiniburn: "#minecraft:infiniburn_overworld",
        respawn_anchor_works: false,
        has_skylight: true,
        bed_works: true,
        effects: "minecraft:overworld",
        min_y: -64,
        height: 384,
        logical_height: 384,
        coodrinate_scale: 1f64,
        ultrawarm: false,
        has_ceiling: false,
    };
}

impl RegistryItem for DimensionType {
    const REGISTRY: &'static str = "minecraft:dimension_type";
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub enum MonsterSpawnLightLevel {
    #[allow(dead_code)]
    Level(i32),
    Range(RangeInclusive<i32>),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct WorldgenBiome {
    pub has_precipitation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<f32>,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    pub downfall: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_modifier: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<BiomeEffects>,
}

impl WorldgenBiome {
    pub const PLAINS: Self = Self {
        has_precipitation: true,
        temperature: 0.8,
        downfall: 0.4,
        depth: None,
        scale: None,
        category: Some("minecraft:plains"),
        temperature_modifier: None,
        effects: Some(BiomeEffects {
            fog_color: 0xc0d8ff,
            sky_color: 0x78a7ff,
            water_color: 0x3f76e4,
            water_fog_color: 0x505330,
            mood_sound: Some(BiomeMoodSound {
                sound: "minecraft:ambient.cave",
                tick_delay: 6000,
                offset: 2.0,
                block_search_extend: 8,
            }),
            additions_sound: None,
            ambient_sound: None,
            foliage_color: None,
            grass_color: None,
            grass_color_modifier: None,
            music: None,
            particle: None,
        }),
    };
}

impl RegistryItem for WorldgenBiome {
    const REGISTRY: &'static str = "minecraft:worldgen/biome";
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct BiomeEffects {
    pub sky_color: i32,
    pub water_fog_color: i32,
    pub fog_color: i32,
    pub water_color: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foliage_color: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grass_color: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grass_color_modifier: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music: Option<BiomeMusic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ambient_sound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additions_sound: Option<BiomeAdditionsSound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood_sound: Option<BiomeMoodSound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub particle: Option<BiomeParticles>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct BiomeMusic {
    pub replace_current_music: bool,
    pub sound: &'static str,
    pub max_delay: i32,
    pub min_delay: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct BiomeAdditionsSound {
    pub sound: &'static str,
    pub tick_chance: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct BiomeMoodSound {
    pub sound: &'static str,
    pub tick_delay: i32,
    pub offset: f64,
    pub block_search_extend: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct BiomeParticles {
    pub probability: f32,
    pub options: BiomeParticleOptions,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct BiomeParticleOptions {
    #[serde(rename = "type")]
    pub particle_type: &'static str,
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "serde")]
pub struct RegistryCodec<'a> {
    #[serde(rename = "minecraft:dimension_type")] pub dimension_type: &'a Registry<DimensionType>,
    #[serde(rename = "minecraft:worldgen/biome")] pub worldgen_biome: &'a Registry<WorldgenBiome>,
    // #[serde(rename = "minecraft:chat_type")] chat_type: ChatType, TODO
}