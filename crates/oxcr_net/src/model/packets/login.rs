use tracing::debug;
use uuid::Uuid;

use crate::{
    error::Error,
    model::{chat::ChatComponent, State, VarInt},
    ser::{deser_cx, Array, Deserialize, FixedStr, Json, Serialize},
};

use super::{Packet, PacketContext};

#[derive(Debug)]
pub struct LoginStart {
    pub name: FixedStr<16>,
    pub uuid: Option<Uuid>,
}

impl Deserialize for LoginStart {
    type Context = PacketContext;
    fn deserialize<'parse, 'a>(
        input: crate::ser::Inp<'parse, 'a, Self::Context>,
    ) -> crate::ser::Resul<'parse, 'a, Self, Self::Context> {
        if input.context().id == Self::ID && input.context().state == Self::STATE {
            let (input, name) = deser_cx(input)?;
            let (input, uuid) = deser_cx(input)?;

            Ok((input, Self { name, uuid }))
        } else {
            let e = Error::InvalidPacketId(input.context().id.0);
            Err((input, e))
        }
    }
}

impl Packet for LoginStart {
    const ID: crate::model::VarInt = VarInt(0x00);
    const STATE: crate::model::State = State::Login;
}

#[derive(Debug)]
pub struct DisconnectLogin {
    pub reason: Json<ChatComponent>,
}

impl Serialize for DisconnectLogin {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.reason.serialize_to(buf)
    }
}

impl Packet for DisconnectLogin {
    const ID: crate::model::VarInt = VarInt(0x00);
    const STATE: crate::model::State = State::Login;
}

/// This packet switches the connection state to [`Play`].
/// # Info
/// Packet ID: 0x02
/// State: Login
/// Bound to: client
///
/// # Layout
/// UUID: Uuid
/// Username: String (16)
/// Number of properties: VarInt ;; number of elements in the next array
/// Properties: Array<Property>
/// [`Play`]: crate::model::State::Play
#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: FixedStr<16>,
    pub properties: Array<Property>,
}

impl Serialize for LoginSuccess {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.uuid.serialize_to(buf);
        self.username.serialize_to(buf);
        self.properties.serialize_to(buf);
    }
}

impl Packet for LoginSuccess {
    const ID: crate::model::VarInt = VarInt(0x02);
    const STATE: crate::model::State = State::Login;
}

/// # Layout
/// Name: String (32767)
/// Value: String (32767)
/// Is signed: Boolean
/// Signature: Optional String (32767) ;; only if `Is signed` is true
#[derive(Debug, Clone)]
pub struct Property {
    pub name: FixedStr<32767>,
    pub value: FixedStr<32767>,
    // hehe
    pub signature: Option<FixedStr<32767>>,
}

impl Serialize for Property {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) {
        self.name.serialize_to(buf);
        self.value.serialize_to(buf);
        self.signature.serialize_to(buf);
    }
}
