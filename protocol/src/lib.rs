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
    fmt_internals,
    closure_track_caller
)]

pub mod error;
pub mod executor;
pub mod logging;
pub mod model;
pub mod nbt;
pub mod nsfr;
pub mod ser;
pub use aott;
pub use bytes;
use bytes::BytesMut;
pub use thiserror;
use tokio_util::sync::CancellationToken;
pub use uuid;
pub mod serde {
    pub use ::serde::*;
    pub use ::serde_json as json;
}
pub use indexmap;
pub use miette;
pub use nu_ansi_term as ansi;
pub use tracing;

/// A macro similar to `vec![$elem; $size]` which returns a boxed array.
///
/// ```rust
///     let _: Box<[u8; 1024]> = box_array![0; 1024];
/// ```
#[macro_export]
macro_rules! box_array {
    ($val:expr ; $len:expr) => {{
        // Use a generic function so that the pointer cast remains type-safe
        fn vec_to_boxed_array<T>(vec: Vec<T>) -> Box<[T; $len]> {
            let boxed_slice = vec.into_boxed_slice();

            let ptr = ::std::boxed::Box::into_raw(boxed_slice) as *mut [T; $len];

            unsafe { Box::from_raw(ptr) }
        }

        vec_to_boxed_array(vec![$val; $len])
    }};
}

pub trait AsyncGet<T: ?Sized + 'static> {
    type Ptr<'a, D: ?Sized + 'static>: Deref<Target = D> + 'a;

    async fn get(&self) -> Self::Ptr<'_, T>;

    async fn get_copy(&self) -> T
    where
        T: Copy,
    {
        let ptr = self.get().await;
        let val = *(ptr.deref());
        drop(ptr);
        val
    }
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

use aott::prelude::{Input, Parser};
use bevy::{app::ScheduleRunnerPlugin, prelude::*, time::TimePlugin};
use error::Result;
use ser::*;
use std::{
    fmt::Debug,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use executor::*;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    select,
    sync::RwLock,
};

use crate::{
    model::{
        packets::{
            Packet, PacketContext, PluginMessage, SerializedPacket, SerializedPacketCompressed,
        },
        State,
    },
    nsfr::when_the_miette,
};

#[derive(Debug)]
pub struct PlayerNet {
    pub send: flume::Sender<(bool, SerializedPacket)>,
    pub recv: flume::Receiver<SerializedPacket>,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub state: RwLock<State>,
    pub compression: Option<usize>,
    pub compressing: Arc<AtomicBool>,
    pub cancellator: CancellationToken,
}

#[derive(Component, Deref, Debug)]
#[deref(forward)]
pub struct PlayerN(pub Arc<PlayerNet>);

unsafe impl Send for PlayerNet {}
unsafe impl Sync for PlayerNet {}

impl PlayerNet {
    pub fn new(
        mut read: OwnedReadHalf,
        mut write: OwnedWriteHalf,
        cancellator: CancellationToken,
        compression: Option<usize>,
    ) -> Self {
        let peer_addr = read.peer_addr().expect("no peer address");
        let local_addr = read.local_addr().expect("no local address");

        let (s_recv, recv) = flume::unbounded();
        let (send, r_send) = flume::unbounded();

        let compressing = Arc::new(AtomicBool::new(false));

        let compressing_ = compressing.clone();
        let send_task = tokio::spawn(async move {
            let Err::<!, _>(e) = async {
                loop {
                    let (compres, packet): (bool, SerializedPacket) = r_send.recv_async().await?;
                    let data = if compression.is_some_and(|_| compres) {
                        trace!("[send]compressing");
                        packet.serialize_compressing(compression)?
                    } else {
                        trace!("[send]not compressing");
                        packet.serialize()?
                    };
                    trace!(?packet, ?data, "sending packet");
                    write.write_all(&data).await?;
                }
            }
            .await;

            drop(write);

            return Err::<!, crate::error::Error>(e);
        });

        let compressing__ = compressing.clone();
        let recv_task = tokio::spawn(async move {
            async {
                let mut buf = BytesMut::new();

                loop {
                    let bufslice = &buf[..];
                    let mut input = Input::new(&bufslice);
                    if let Some(packet) = match if compressing__.load(Ordering::SeqCst) {
                        SerializedPacketCompressed::deserialize
                            .parse_with(&mut input)
                            .map(SerializedPacket::from)
                    } else {
                        SerializedPacket::deserialize.parse_with(&mut input)
                    } {
                        Ok(packet) => Some(packet),
                        Err(crate::error::Error::Ser(
                            crate::ser::SerializationError::UnexpectedEof { .. },
                        )) => None,
                        Err(other) => return Err(other),
                    } {
                        let offset = input.offset;
                        drop((bufslice, input));
                        buf = buf.split_off(offset);
                        s_recv.send_async(packet).await?;
                    } else {
                        if read.read_buf(&mut buf).await? == 0 {
                            if buf.is_empty() {
                                break;
                            } else {
                                return Err(crate::error::Error::ConnectionEnded);
                            }
                        }
                    }
                }

                Ok(())
            }
            .await?;

            drop(read);

            Ok::<(), crate::error::Error>(())
        });

        let cancellator_ = cancellator.clone();
        tokio::spawn(async move {
            let cancellator = cancellator_;
            select! {
                Ok(thimg) = recv_task => {
                    match when_the_miette(thimg) {
                        Ok(()) => info!(%peer_addr, "Disconnected (connection ended)"),
                        Err(error) => info!(%peer_addr, ?error, "Disconnected (connection ended)")
                    }
                    cancellator.cancel();
                }
                Ok(Err(error)) = send_task => {
                    error!(%peer_addr, error=?when_the_miette(Err::<!, _>(error)), "Disconnected (due to error)");
                    cancellator.cancel();
                }
            }
        });

        Self {
            send,
            recv,
            peer_addr,
            local_addr,
            state: RwLock::new(State::Handshaking),
            compression,
            compressing,
            cancellator,
        }
    }

    /// Reads a packet.
    pub async fn recv_packet<T: Packet + Deserialize<Context = PacketContext> + Debug>(
        &self,
    ) -> Result<T> {
        let tnov = std::any::type_name::<T>();
        if self.recv.is_disconnected() {
            trace!(packet=tnov, addr=%self.peer_addr, "receiving packet failed - disconnected");
            return Err(crate::error::Error::ConnectionEnded);
        }
        let packet = self.recv.recv_async().await?;
        let state = *self.state.read().await;
        trace!(%self.peer_addr, ?packet, ?state, "Received packet");
        let result = packet.try_deserialize(state);
        trace!(?result, %self.peer_addr, "Deserialized packet");
        result
    }

    /// Writes a packet.
    pub async fn send_packet<T: Packet + Serialize + Debug>(&self, packet: T) -> Result<()> {
        if self.send.is_disconnected() {
            trace!(?packet, addr=%self.peer_addr, "sending packet failed - disconnected");
            return Err(crate::error::Error::ConnectionEnded);
        }
        let spack = SerializedPacket::new_ref(&packet)?;
        trace!(?packet, addr=%self.peer_addr, ?spack, "Sending packet");
        Ok(self
            .send
            .send_async((self.compressing.load(Ordering::SeqCst), spack))
            .await?)
    }

    /// Sends a plugin message.
    /// Equivalent to `self.send_packet(PluginMessage { channel, data: data.serialize() })`
    pub async fn plugin_message<T: Serialize + Debug>(
        &self,
        channel: Identifier,
        data: T,
    ) -> Result<()> {
        trace!(?channel, ?data, %self.peer_addr, "Sending plugin message");
        self.send_packet(PluginMessage {
            channel,
            data: data.serialize()?,
        })
        .await
    }

    /// Receives a packet and tries to deserialize it.
    /// If deserialization fails, returns the packet as-is, allowing for further attempt at using the packet.
    pub async fn try_recv_packet<T: Packet + Deserialize<Context = PacketContext> + Debug>(
        &self,
    ) -> Result<Result<T, (State, SerializedPacket)>> {
        let packet = self.recv.recv_async().await?;
        let state = *self.state.read().await;
        trace!(%self.peer_addr, ?packet, ?state, "trying to receive packet");

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
