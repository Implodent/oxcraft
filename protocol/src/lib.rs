#![feature(
    try_blocks,
    associated_type_defaults,
    decl_macro,
    iterator_try_collect,
    maybe_uninit_array_assume_init,
    const_mut_refs,
    const_maybe_uninit_write,
    const_maybe_uninit_array_assume_init,
    async_fn_in_trait,
    exhaustive_patterns,
    never_type,
    fmt_internals
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
pub use indexmap;
pub use miette;
pub use tracing;

pub async fn rwlock_set<T: 'static>(rwlock: &RwLock<T>, value: T) {
    rwlock.set(value).await
}

pub trait AsyncGet<T: ?Sized + 'static> {
    type Ptr<'a, D: ?Sized + 'static>: Deref<Target = D> + 'a;

    async fn get(&self) -> Self::Ptr<'_, T>;
}

pub trait AsyncSet<T: ?Sized + 'static>: AsyncGet<T> {
    type PtrMut<'a, D: ?Sized + 'static>: DerefMut<Target = D> + 'a;

    async fn set(&self, value: T)
    where
        T: Sized,
    {
        let mut mu = self.get_mut().await;
        *mu.deref_mut() = value;
    }

    async fn get_mut(&self) -> Self::PtrMut<'_, T>;

    async fn replace(&self, value: T) -> T
    where
        T: Sized,
    {
        let mut ptr = self.get_mut().await;
        std::mem::replace(ptr.deref_mut(), value)
    }

    async fn compare_swap(&self, compare_to: &T, swap_with: T)
    where
        T: Sized + PartialEq,
    {
        let got = self.get().await;
        if got.deref() == compare_to {
            drop(got);

            self.set(swap_with).await;
        }
    }
}

impl<T: ?Sized + 'static> AsyncGet<T> for RwLock<T> {
    type Ptr<'a, D: ?Sized + 'static> = tokio::sync::RwLockReadGuard<'a, D>;

    async fn get(&self) -> Self::Ptr<'_, T> {
        self.read().await
    }
}

impl<T: ?Sized + 'static> AsyncSet<T> for RwLock<T> {
    type PtrMut<'a, D: ?Sized + 'static> = tokio::sync::RwLockWriteGuard<'a, D>;

    async fn get_mut(&self) -> Self::PtrMut<'_, T> {
        self.write().await
    }
}

use aott::prelude::Parser;
use bevy::{app::ScheduleRunnerPlugin, prelude::*, time::TimePlugin};
use bytes::BytesMut;
use error::Result;
use ser::*;
use std::{
    fmt::Debug,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

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
    pub compression: RwLock<Option<usize>>,
    pub _explode: mpsc::Sender<()>,
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
            let Err::<!, _>(e) = async {
                loop {
                    let packet: SerializedPacket = r_send.recv_async().await?;
                    let data = packet.serialize()?;
                    write.write_all(&data).await?;
                }
            }
            .await;

            drop(write);

            return Err::<!, crate::error::Error>(e);
        });
        let recv_task = tokio::spawn(async move {
            async {
                let mut buf = BytesMut::new();

                loop {
                    let read_bytes = read.read_buf(&mut buf).await?;
                    if read_bytes == 0 {
                        return Ok::<(), crate::error::Error>(());
                    }
                    let spack = SerializedPacket::deserialize.parse(buf.as_ref())?;
                    s_recv.send_async(spack).await?;
                    buf.clear();
                }
            }
            .await?;

            drop(read);

            Ok::<(), crate::error::Error>(())
        });

        let shitshit = shit.clone();
        tokio::spawn(async move {
            let shit = shitshit;
            select! {
                Ok(thimg) = recv_task => {
                    match thimg {
                        Ok(()) => info!(%peer_addr, "Disconnected (connection ended)"),
                        Err(error) => info!(%peer_addr, ?error, "Disconnected (connection ended)")
                    }
                    shit.send(()).await.unwrap_or_else(|_| error!("disconnect failed (already disconnected)"));
                }
                Ok(Err(error)) = send_task => {
                    error!(%peer_addr, ?error, "Disconnected (due to error)");
                    shit.send(()).await.unwrap_or_else(|_| error!("disconnect failed (already disconnected)"));
                }
            }
        });

        Self {
            send,
            recv,
            peer_addr,
            local_addr,
            state: RwLock::new(State::Handshaking),
            compression: RwLock::new(None),
            _explode: shit,
        }
    }

    /// Reads a packet.
    pub async fn recv_packet<T: Packet + Deserialize<Context = PacketContext> + Debug>(
        &self,
    ) -> Result<T> {
        let tnov = std::any::type_name::<T>();
        if self.recv.is_disconnected() {
            debug!(packet=tnov, addr=%self.peer_addr, "receiving packet failed - disconnected");
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
            debug!(?packet, addr=%self.peer_addr, "sending packet failed - disconnected");
            return Err(crate::error::Error::ConnectionEnded);
        }
        let spack = SerializedPacket::new_ref(&packet)?;
        debug!(?packet, addr=%self.peer_addr, ?spack, "Sending packet");
        Ok(self.send.send(spack)?)
    }

    /// Sends a plugin message.
    /// Equivalent to `self.send_packet(PluginMessage { channel, data: data.serialize() })`
    pub fn plugin_message<T: Serialize + Debug>(&self, channel: Identifier, data: T) -> Result<()> {
        debug!(?channel, ?data, %self.peer_addr, "Sending plugin message");
        self.send_packet(PluginMessage {
            channel,
            data: data.serialize()?,
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
