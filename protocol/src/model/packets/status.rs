use crate::{
    model::{chat::ChatComponent, State, VarInt},
    ser::{Deserialize, Extra, Json, Serialize},
};
use aott::parser::Parser;
use aott::{bytes as b, prelude::parser};
use bytes::BufMut;
use serde_derive::{Deserialize, Serialize};

use super::{Packet, PacketContext};

#[derive(Debug)]
pub struct StatusRequest;
impl Deserialize for StatusRequest {
    type Context = PacketContext;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        if input.context().id.0 == 0x00 && input.context().state == State::Status {
            Ok(Self)
        } else {
            let id = input.context().id.0;
            Err(crate::error::Error::InvalidPacketId(id))
        }
    }
}
impl Packet for StatusRequest {
    const ID: crate::model::VarInt = VarInt(0x00);
    const STATE: crate::model::State = State::Status;
}

#[derive(Debug)]
pub struct StatusResponse {
    pub json_response: Json<StatusResponseJson>,
}

impl Packet for StatusResponse {
    const ID: crate::model::VarInt = VarInt(0x00);
    const STATE: crate::model::State = State::Status;
}

impl Serialize for StatusResponse {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.json_response.serialize_to(buf);
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponseJson {
    pub version: Version,
    pub players: Players,
    pub description: ChatComponent,
    pub favicon: String,
    pub enforces_secure_chat: bool,
    pub previews_chat: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Players {
    pub max: i64,
    pub online: i64,
    pub sample: Vec<Sample>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub name: String,
    pub id: String,
}

#[derive(Debug)]
pub struct PingRequest {
    pub payload: i64,
}

impl Packet for PingRequest {
    const ID: crate::model::VarInt = VarInt(0x01);
    const STATE: crate::model::State = State::Status;
}
impl Deserialize for PingRequest {
    type Context = PacketContext;

    #[parser(extras = "Extra<Self::Context>")]
    fn deserialize(input: &[u8]) -> Self {
        b::number::big::i64
            .map(|payload| Self { payload })
            .parse_with(input)
    }
}

#[derive(Debug)]
pub struct PongResponse {
    pub payload: i64,
}

impl Packet for PongResponse {
    const ID: crate::model::VarInt = VarInt(0x01);
    const STATE: crate::model::State = State::Status;
}
impl Serialize for PongResponse {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        buf.put_i64(self.payload);
    }
}
