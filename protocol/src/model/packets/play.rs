use crate::{
    model::{chat::ChatComponent, Difficulty, State, VarInt},
    nbt::Nbt,
    ser::*,
    PacketContext,
};
use std::ptr;

use aott::primitive::one_of;
use bytes::BufMut;
use indexmap::IndexMap;

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
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        self.reason.serialize_to(buf)
    }
}

#[derive(Debug, Clone)]
pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub game_mode: GameMode,
    pub prev_game_mode: PreviousGameMode,
    pub dimension_names: Array<Identifier>,
    pub registry_codec: IndexMap<String, Nbt>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl Serialize for GameMode {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        buf.put_u8(*self as u8);
        Ok(())
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
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        buf.put_i8(match self {
            Self::Undefined => -1,
            Self::Normal(gamemode) => *gamemode as i8,
        });
        Ok(())
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
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
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
