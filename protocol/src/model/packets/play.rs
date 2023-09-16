use crate::{
    model::{chat::ChatComponent, Difficulty, State, VarInt},
    nbt::NbtJson,
    ser::*,
    PacketContext,
};
use std::ptr;

use aott::primitive::one_of;
use bytes::BufMut;

use super::Packet;

#[derive(Debug)]
pub struct DisconnectPlay {
    pub reason: Json<ChatComponent>,
}

impl Packet for DisconnectPlay {
    const ID: crate::model::VarInt = VarInt(0x1A);
    const STATE: crate::model::State = State::Play;
}

impl Serialize for DisconnectPlay {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.reason.serialize_to(buf);
    }
}

#[derive(Debug, Clone)]
pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub game_mode: GameMode,
    pub prev_game_mode: PreviousGameMode,
    pub dimension_names: Array<Identifier>,
    pub registry_codec: NbtJson<serde_json::Value>,
    pub dimension_type: Identifier,
    pub dimension_name: Identifier,
    pub hashed_seed: i64,
    pub max_players: VarInt,
    pub view_distance: VarInt,
    pub simulation_distance: VarInt,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death_location: Option<DeathLocation>,
    pub portal_cooldown: VarInt,
}

impl Packet for LoginPlay {
    const ID: crate::model::VarInt = VarInt(0x28);
    const STATE: crate::model::State = State::Play;
}

impl_ser!(|PacketContext| LoginPlay => [
    entity_id,
    is_hardcore,
    game_mode,
    prev_game_mode,
    dimension_names,
    registry_codec,
    dimension_type,
    dimension_name,
    hashed_seed,
    max_players,
    view_distance,
    simulation_distance,
    reduced_debug_info,
    enable_respawn_screen,
    is_debug,
    is_flat,
    death_location,
    portal_cooldown
]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathLocation {
    pub dimension: Identifier,
    pub location: Position,
}

impl_ser!(DeathLocation => [
    dimension, location
]);

pub mod json {
    use serde::{Deserialize, Serialize};

    pub const CODEC_120: &'static str = include_str!("./login_play_registry_codec_120.json");

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RegistryCodec {
        #[serde(rename = "minecraft:chat_type")]
        pub minecraft_chat_type: ChatTypeRegistry,
        #[serde(rename = "minecraft:damage_type")]
        pub minecraft_damage_type: DamageTypeRegistry,
        #[serde(rename = "minecraft:dimension_type")]
        pub minecraft_dimension_type: DimensionTypeRegistry,
        #[serde(rename = "minecraft:trim_material")]
        pub minecraft_trim_material: TrimMaterialRegistry,
        #[serde(rename = "minecraft:trim_pattern")]
        pub minecraft_trim_pattern: TrimPatternRegistry,
        #[serde(rename = "minecraft:worldgen/biome")]
        pub minecraft_worldgen_biome: WorldgenBiomeRegistry,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ChatTypeRegistry {
        #[serde(rename = "type")]
        pub type_field: String,
        pub value: Vec<ChatTypeObject>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ChatTypeObject {
        pub element: ChatType,
        pub id: i64,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ChatType {
        pub chat: Chat,
        pub narration: Narration,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Chat {
        pub parameters: Vec<String>,
        #[serde(rename = "translation_key")]
        pub translation_key: String,
        pub style: Option<ChatStyle>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ChatStyle {
        pub color: String,
        pub italic: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Narration {
        pub parameters: Vec<String>,
        #[serde(rename = "translation_key")]
        pub translation_key: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DamageTypeRegistry {
        #[serde(rename = "type")]
        pub type_field: String,
        pub value: Vec<DamageType>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DamageType {
        pub element: DamageTypeElement,
        pub id: i64,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DamageTypeElement {
        pub exhaustion: f64,
        #[serde(rename = "message_id")]
        pub message_id: String,
        pub scaling: String,
        #[serde(rename = "death_message_type")]
        pub death_message_type: Option<String>,
        pub effects: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DimensionTypeRegistry {
        #[serde(rename = "type")]
        pub type_field: String,
        pub value: Vec<DimensionType>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DimensionType {
        pub element: Dimension,
        pub id: i64,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dimension {
        #[serde(rename = "ambient_light")]
        pub ambient_light: f64,
        #[serde(rename = "bed_works")]
        pub bed_works: i64,
        #[serde(rename = "coordinate_scale")]
        pub coordinate_scale: i64,
        pub effects: String,
        #[serde(rename = "has_ceiling")]
        pub has_ceiling: i64,
        #[serde(rename = "has_raids")]
        pub has_raids: i64,
        #[serde(rename = "has_skylight")]
        pub has_skylight: i64,
        pub height: i64,
        pub infiniburn: String,
        #[serde(rename = "logical_height")]
        pub logical_height: i64,
        #[serde(rename = "min_y")]
        pub min_y: i64,
        #[serde(rename = "monster_spawn_block_light_limit")]
        pub monster_spawn_block_light_limit: i64,
        #[serde(rename = "monster_spawn_light_level")]
        pub monster_spawn_light_level: ChatTypeObject,
        pub natural: i64,
        #[serde(rename = "piglin_safe")]
        pub piglin_safe: i64,
        #[serde(rename = "respawn_anchor_works")]
        pub respawn_anchor_works: i64,
        pub ultrawarm: i64,
        #[serde(rename = "fixed_time")]
        pub fixed_time: Option<i64>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrimMaterialRegistry {
        #[serde(rename = "type")]
        pub type_field: String,
        pub value: Vec<TrimMaterialObject>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrimMaterialObject {
        pub element: TrimMaterial,
        pub id: i64,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrimMaterial {
        #[serde(rename = "asset_name")]
        pub asset_name: String,
        pub description: Description,
        pub ingredient: String,
        #[serde(rename = "item_model_index")]
        pub item_model_index: f64,
        #[serde(rename = "override_armor_materials")]
        pub override_armor_materials: Option<OverrideArmorMaterials>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Description {
        pub color: String,
        pub translate: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OverrideArmorMaterials {
        pub netherite: Option<String>,
        pub iron: Option<String>,
        pub gold: Option<String>,
        pub diamond: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrimPatternRegistry {
        #[serde(rename = "type")]
        pub type_field: String,
        pub value: Vec<TrimPatternObject>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrimPatternObject {
        pub element: TrimPattern,
        pub id: i64,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrimPattern {
        #[serde(rename = "asset_id")]
        pub asset_id: String,
        pub description: Description2,
        #[serde(rename = "template_item")]
        pub template_item: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Description2 {
        pub translate: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WorldgenBiomeRegistry {
        #[serde(rename = "type")]
        pub type_field: String,
        pub value: Vec<WorldgenBiomeObject>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WorldgenBiomeObject {
        pub element: Biome,
        pub id: i64,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Biome {
        pub downfall: f64,
        pub effects: Effects,
        #[serde(rename = "has_precipitation")]
        pub has_precipitation: i64,
        pub temperature: f64,
        #[serde(rename = "temperature_modifier")]
        pub temperature_modifier: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Effects {
        #[serde(rename = "fog_color")]
        pub fog_color: i64,
        #[serde(rename = "foliage_color")]
        pub foliage_color: Option<i64>,
        #[serde(rename = "grass_color")]
        pub grass_color: Option<i64>,
        #[serde(rename = "mood_sound")]
        pub mood_sound: MoodSound,
        pub music: Option<Music>,
        #[serde(rename = "sky_color")]
        pub sky_color: i64,
        #[serde(rename = "water_color")]
        pub water_color: i64,
        #[serde(rename = "water_fog_color")]
        pub water_fog_color: i64,
        #[serde(rename = "additions_sound")]
        pub additions_sound: Option<AdditionsSound>,
        #[serde(rename = "ambient_sound")]
        pub ambient_sound: Option<String>,
        pub particle: Option<Particle>,
        #[serde(rename = "grass_color_modifier")]
        pub grass_color_modifier: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MoodSound {
        #[serde(rename = "block_search_extent")]
        pub block_search_extent: i64,
        pub offset: i64,
        pub sound: String,
        #[serde(rename = "tick_delay")]
        pub tick_delay: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Music {
        #[serde(rename = "max_delay")]
        pub max_delay: i64,
        #[serde(rename = "min_delay")]
        pub min_delay: i64,
        #[serde(rename = "replace_current_music")]
        pub replace_current_music: i64,
        pub sound: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AdditionsSound {
        pub sound: String,
        #[serde(rename = "tick_chance")]
        pub tick_chance: f64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Particle {
        pub options: ParticleOptions,
        pub probability: f64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ParticleOptions {
        #[serde(rename = "type")]
        pub type_field: String,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl Serialize for GameMode {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        buf.put_u8(*self as u8);
    }
}

impl Deserialize for GameMode {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let byte = one_of([0x0, 0x1, 0x2, 0x3])(input)?;
        Ok(unsafe { *ptr::addr_of!(byte).cast() })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum PreviousGameMode {
    Undefined = -1,
    Normal(GameMode),
}
impl Serialize for PreviousGameMode {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        buf.put_i8(match self {
            Self::Undefined => -1,
            Self::Normal(gamemode) => *gamemode as i8,
        });
    }
}
impl Deserialize for PreviousGameMode {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let byte = aott::bytes::number::big::i8
            .filter(|g| (-1..=3).contains(g))
            .parse_with(input)?;
        Ok(unsafe { *ptr::addr_of!(byte).cast() })
    }
}

#[derive(Debug, Clone)]
pub struct ChangeDifficulty {
    pub difficulty: Difficulty,
    pub difficulty_locked: bool,
}

impl_ser!(|PacketContext| ChangeDifficulty => [difficulty, difficulty_locked]);
impl Packet for ChangeDifficulty {
    const ID: crate::model::VarInt = VarInt(0x0c);
    const STATE: crate::model::State = State::Play;
}

#[derive(Debug, Clone)]
pub struct PlayerAbilities {
    pub flags: Abilities,
    pub flying_speed: f32,
    pub fov_modifier: f32,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Abilities: u8 {
        const INVULNERABLE = 0x01;
        const FLYING = 0x02;
        const ALLOW_FLYING = 0x04;
        const CREATIVE_MODE = 0x08;
    }
}

impl Serialize for Abilities {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.bits().serialize_to(buf)
    }
}
impl Deserialize for Abilities {
    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        Self::from_bits(input.next()?).ok_or(crate::error::Error::InvalidBitFlags)
    }
}
impl_ser!(|PacketContext| PlayerAbilities => [flags, flying_speed, fov_modifier]);
impl Packet for PlayerAbilities {
    const ID: crate::model::VarInt = VarInt(0x34);
    const STATE: crate::model::State = State::Play;
}
