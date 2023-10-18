pub mod chat;
pub mod packets;
pub mod registry;
mod varint;
use self::registry::RegistryItem;
use aott::primitive::filter;
use bytes::BufMut;
use std::{ops::RangeInclusive, ptr};
pub use varint::*;

use crate::ser::{Deserialize, Serialize, Type};
pub mod item;

pub const MAX_PACKET_DATA: usize = 0x1FFFFF;
pub const PROTOCOL_VERSION: i32 = 763;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Handshaking,
    Status,
    Login,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Difficulty {
    #[default]
    Peaceful = 0,
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl Serialize for Difficulty {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        buf.put_u8(*self as _);
        Ok(())
    }
}

impl Deserialize for Difficulty {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], crate::ser::Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, crate::ser::Extra<Self::Context>> {
        let byte = filter(|x| matches!(x, 0..=3), Type::Difficulty)(input)?;
        Ok(unsafe { *ptr::addr_of!(byte).cast() })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct DimensionType {
    pub piglin_safe: bool,
    pub has_raids: bool,
    pub monster_spawn_light_level: MonsterSpawnLightLevel,
    pub monster_spawn_block_light_limit: i32,
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
    pub coordinate_scale: f64,
    pub ultrawarm: bool,
    pub has_ceiling: bool,
}

impl DimensionType {
    pub const OVERWORLD: Self = Self {
        ambient_light: 0.0,
        bed_works: true,
        coordinate_scale: 1.0,
        effects: "minecraft:overworld",
        has_ceiling: false,
        has_raids: true,
        has_skylight: true,
        height: 384,
        infiniburn: "#minecraft:infiniburn_overworld",
        logical_height: 384,
        min_y: -64,
        monster_spawn_block_light_limit: 0,
        monster_spawn_light_level: MonsterSpawnLightLevel::Level(0),
        natural: true,
        piglin_safe: false,
        respawn_anchor_works: false,
        ultrawarm: false,
        fixed_time: None,
    };
}

impl RegistryItem for DimensionType {
    const REGISTRY: &'static str = "minecraft:dimension_type";
}

#[derive(Debug, Clone)]
pub enum MonsterSpawnLightLevel {
    #[allow(dead_code)]
    Level(i32),
    Range(RangeInclusive<i32>),
}

impl serde::Serialize for MonsterSpawnLightLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Level(level) => serializer.serialize_i32(*level),
            Self::Range(range) => {
                #[derive(serde::Serialize)]
                #[serde(crate = "serde")]
                struct MSLLRange {
                    #[serde(rename = "type")]
                    ty: &'static str,
                    value: MSLLRangeRange,
                }

                #[derive(serde::Serialize)]
                #[serde(crate = "serde")]
                struct MSLLRangeRange {
                    max_inclusive: i32,
                    min_inclusive: i32,
                }

                let r = MSLLRange {
                    ty: "minecraft:uniform",
                    value: MSLLRangeRange {
                        max_inclusive: *range.end(),
                        min_inclusive: *range.start(),
                    },
                };

                serde::Serialize::serialize(&r, serializer)
            }
        }
    }
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct ChatType;

impl RegistryItem for ChatType {
    const REGISTRY: &'static str = "minecraft:chat_type";
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "serde")]
pub struct DamageType {
    pub exhaustion: f32,
    pub message_id: &'static str,
    pub scaling: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub death_message_type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<&'static str>,
}

macro declare_thing($vis:vis, $($name:ident { $($field:ident : $value:expr),* }),*) {
    $($vis const $name: Self = Self {
        $($field: $value,)*
        ..Self::EMPTY
    };)*
}

impl DamageType {
    const EMPTY: Self = Self {
        exhaustion: 0.0,
        message_id: "",
        scaling: "",
        death_message_type: None,
        effects: None,
    };

    declare_thing! {pub,
        ARROW {
            exhaustion: 0.1,
            message_id: "arrow",
            scaling: "when_caused_by_living_non_player"
        },
        BAD_RESPAWN_POINT {
            exhaustion: 0.1,
            message_id: "badRespawnPoint",
            scaling: "always",
            death_message_type: Some("intentional_game_design")
        }
    }
}

impl RegistryItem for DamageType {
    const REGISTRY: &'static str = "minecraft:damage_type";
}
