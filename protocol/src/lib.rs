#![feature(
    try_blocks,
    associated_type_defaults,
    decl_macro,
    iterator_try_collect,
    maybe_uninit_array_assume_init,
    const_mut_refs,
    const_maybe_uninit_write,
    const_maybe_uninit_array_assume_init
)]

pub mod error;
pub mod executor;
pub mod model;
pub mod nbt;
pub mod nsfr;
pub mod ser;
pub use aott;
pub use bytes;
pub use thiserror;
pub use uuid;
pub mod serde {
    pub use ::serde::*;
    pub use ::serde_json as json;
}

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

pub async fn rwlock_set<T>(rwlock: &RwLock<T>, value: T) {
    let mut w = rwlock.write().await;
    *w = value;
}

use aott::prelude::Parser;
use bevy::{app::ScheduleRunnerPlugin, prelude::*, time::TimePlugin};
use bytes::BytesMut;
use error::Result;
use ser::*;
use std::{fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use executor::*;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    select,
    sync::{mpsc, RwLock},
};

use crate::model::{
    packets::{Packet, PacketContext, PluginMessage, SerializedPacket},
    State,
};

#[derive(Debug)]
pub struct PlayerNet {
    pub send: flume::Sender<SerializedPacket>,
    pub recv: flume::Receiver<SerializedPacket>,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub state: RwLock<State>,
}

#[derive(Component, Deref, Debug)]
#[deref(forward)]
pub struct PlayerN(pub Arc<PlayerNet>);

unsafe impl Send for PlayerNet {}
unsafe impl Sync for PlayerNet {}

impl PlayerNet {
    pub fn new(mut read: OwnedReadHalf, mut write: OwnedWriteHalf, shit: mpsc::Sender<()>) -> Self {
        let peer_addr = read.peer_addr().expect("no peer address");
        let local_addr = read.local_addr().expect("no local address");
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
                Ok(thimg) = recv_task => {
                    match thimg {
                        Ok(()) =>info!(%peer_addr, "Disconnected (connection ended)"),
                        Err(error) => error!(%peer_addr, ?error, "Disconnected (connection ended)")
                    }
                    shit.send(()).await.expect("the fuck????");
                }
                Ok(Err(error)) = send_task => {
                    error!(%peer_addr, ?error, "Disconnected (due to error)");
                    shit.send(()).await.expect("THE FUCK????");
                }
            }
        });
        Self {
            send,
            recv,
            peer_addr,
            local_addr,
            state: RwLock::new(State::Handshaking),
        }
    }

    /// Reads a packet.
    pub async fn recv_packet<T: Packet + Deserialize<Context = PacketContext> + Debug>(
        &self,
    ) -> Result<T> {
        if self.recv.is_disconnected() {
            return Err(crate::error::Error::ConnectionEnded);
        }
        let packet = self.recv.recv_async().await?;
        let state = *self.state.read().await;
        debug!(%self.peer_addr, ?packet, ?state, "Received packet");
        let result = packet.try_deserialize(state);
        debug!(?result, %self.peer_addr, "Deserialized packet");
        result
    }

    /// Writes a packet.
    pub fn send_packet<T: Packet + Serialize + Debug>(&self, packet: T) -> Result<()> {
        if self.send.is_disconnected() {
            return Err(crate::error::Error::ConnectionEnded);
        }
        debug!(?packet, addr=%self.peer_addr, "Sending packet");
        Ok(self.send.send(SerializedPacket::new(packet))?)
    }

    /// Sends a plugin message.
    /// Equivalent to `self.send_packet(PluginMessage { channel, data: data.serialize() })`
    pub fn plugin_message<T: Serialize + Debug>(&self, channel: Identifier, data: T) -> Result<()> {
        debug!(?channel, ?data, %self.peer_addr, "Sending plugin message");
        self.send_packet(PluginMessage {
            channel,
            data: data.serialize(),
        })
    }

    /// Receives a packet and tries to deserialize it.
    /// If deserialization fails, returns the packet as-is, allowing for further attempt at using the packet.
    pub async fn try_recv_packet<T: Packet + Deserialize<Context = PacketContext> + Debug>(
        &self,
    ) -> Result<Result<T, (State, SerializedPacket)>> {
        let packet = self.recv.recv_async().await?;
        let state = *self.state.read().await;
        debug!(%self.peer_addr, ?packet, ?state, "trying to receive packet");

        match packet.try_deserialize(state) {
            Ok(deserialized) => {
                debug!(%self.peer_addr, ?deserialized, "Deserialized packet successfully");
                Ok(Ok(deserialized))
            }
            Err(error) => {
                debug!(%self.peer_addr, ?packet, %error, "Deserialization errored");
                Ok(Err((state, packet)))
            }
        }
    }
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TokioTasksPlugin::default(),
            TypeRegistrationPlugin,
            TimePlugin,
            ScheduleRunnerPlugin::run_loop(Duration::from_millis(50)),
        ));
    }
}
