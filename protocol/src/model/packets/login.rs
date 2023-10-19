use aott::{
    prelude::{parser, Parser},
    primitive::just,
};
use uuid::Uuid;

use crate::{
    error::Error,
    model::{chat::ChatComponent, State, VarInt},
    ser::{deser_cx, impl_ser, no_context, Deserialize, Extra, FixedStr, Json, Serialize},
};

use super::{Packet, PacketContext};

#[derive(Debug)]
pub struct LoginStart {
    pub name: FixedStr<16>,
    pub uuid: Option<Uuid>,
}

impl_ser!(|PacketContext| LoginStart => [name, uuid]);

impl Packet for LoginStart {
    const ID: crate::model::VarInt = VarInt(0x00);
    const STATE: crate::model::State = State::Login;
}

#[derive(Debug)]
pub struct DisconnectLogin {
    pub reason: Json<ChatComponent>,
}

impl Serialize for DisconnectLogin {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
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
}

impl Serialize for LoginSuccess {
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        self.uuid.serialize_to(buf)?;
        self.username.serialize_to(buf)?;
        VarInt(0).serialize_to(buf)?;
        Ok(())
    }
}

impl Deserialize for LoginSuccess {
    type Context = PacketContext;

    fn deserialize<'a>(
        input: &mut aott::prelude::Input<&'a [u8], Extra<Self::Context>>,
    ) -> aott::PResult<&'a [u8], Self, Extra<Self::Context>> {
        let uuid = no_context(Uuid::deserialize)(input)?;
        let username = no_context(FixedStr::deserialize)(input)?;

        just(0x0)(input)?;

        Ok(Self { uuid, username })
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
    fn serialize_to(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::error::Error> {
        self.name.serialize_to(buf)?;
        self.value.serialize_to(buf)?;
        self.signature.serialize_to(buf)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SetCompression {
    pub threshold: VarInt,
}

impl_ser!(|PacketContext| SetCompression => [threshold]);

impl Packet for SetCompression {
    const ID: VarInt = VarInt(0x04);
    const STATE: State = State::Login;
}
