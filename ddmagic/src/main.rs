use std::{
    env,
    net::SocketAddr,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use oxcr_protocol::{
    error::Result,
    logging::CraftLayer,
    miette::{self, ensure, miette, IntoDiagnostic},
    model::{
        packets::{
            handshake::{Handshake, HandshakeNextState},
            login::{LoginStart, LoginSuccess, SetCompression},
            status::{PingRequest, PongResponse, StatusRequest, StatusResponse},
        },
        VarInt,
    },
    ser::FixedStr,
    PlayerNet,
};
use tokio::net::{TcpSocket, TcpStream};
use tokio_util::sync::CancellationToken;
use tracing::*;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

async fn connect(addr: SocketAddr) -> Result<TcpStream> {
    info!("trying to connect to {addr}...");
    let sock = TcpSocket::new_v4()?;
    match tokio::time::timeout(Duration::from_secs(10), sock.connect(addr)).await {
        Ok(guh) => guh.map_err(Into::into),
        Err(meh) => {
            error!("connection to {addr} timed out...");
            Err(meh.into())
        }
    }
}

async fn status(addr: SocketAddr) -> Result<u32> {
    let stream = connect(addr).await?;

    let (read, write) = stream.into_split();
    let net = Arc::new(PlayerNet::new(
        read,
        write,
        CancellationToken::new(),
        Arc::new(AtomicIsize::new(-1)),
    ));

    net.send_packet(Handshake {
        addr: FixedStr::from_string(&addr.ip().to_string())
            .expect("address was longer than 255 chars"),
        protocol_version: VarInt(754),
        port: addr.port(),
        next_state: HandshakeNextState::Status,
    })
    .await?;

    net.send_packet(StatusRequest).await?;

    net.recv_packet::<StatusResponse>().await?;

    net.send_packet(PingRequest {
        payload: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap(),
    })
    .await?;

    let PongResponse { payload: pong } = net.recv_packet().await?;

    Ok((SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
        - Duration::from_millis(pong.try_into().unwrap()))
    .as_millis()
    .try_into()
    .unwrap())
}

async fn one(index: usize, addr: SocketAddr) -> Result<()> {
    info!("task #{index} launched");

    let ping = status(addr).await?;

    info!("[#{index}] ping: {ping}");

    let stream = connect(addr).await?;

    let (read, write) = stream.into_split();
    let net = Arc::new(PlayerNet::new(
        read,
        write,
        CancellationToken::new(),
        Arc::new(AtomicIsize::new(-1)),
    ));

    net.send_packet(Handshake {
        addr: FixedStr::from_string(&addr.ip().to_string())
            .expect("address was longer than 255 chars"),
        protocol_version: VarInt(754),
        port: addr.port(),
        next_state: HandshakeNextState::Login,
    })
    .await?;

    net.send_packet(LoginStart {
        name: FixedStr::from_string(&format!("yeetus{index}")).unwrap(),
        uuid: None,
    })
    .await?;

    let LoginSuccess { uuid, username } = match net.try_recv_packet::<SetCompression>().await? {
        Ok(SetCompression {
            threshold: VarInt(threshold),
        }) => {
            net.compression.store(threshold as isize, Ordering::SeqCst);
            net.compressing.store(true, Ordering::SeqCst);

            net.recv_packet::<LoginSuccess>().await
        }
        Err((state, pack)) => pack.try_deserialize::<LoginSuccess>(state),
    }?;

    info!("[#{index}] I am uuid {uuid}, named {username}");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), miette::Report> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_env("OXCR_LOG"))
        .with(CraftLayer)
        .init();

    miette::set_panic_hook();

    let amount = 10;
    let mut enva = env::args();
    ensure!(enva.next().is_some(), "no args");

    let addr = enva
        .next()
        .ok_or_else(|| miette!("give me ip addres or i explond"))?
        .parse()
        .into_diagnostic()?;

    info!("DDOSing {addr}");

    let mut ayfj = Vec::with_capacity(amount);

    for index in 0..amount {
        trace!("tasking #{index}");
        ayfj.push(tokio::spawn(async move {
            match one(index, addr).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("{e:?}");
                    Err(e)
                }
            }
        }));
    }

    for task in ayfj.into_iter() {
        task.await.unwrap()?;
    }

    Ok(())
}
