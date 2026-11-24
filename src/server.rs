use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::protocol::{self, Intent, MAX_PACKET};
use crate::response::Responses;
use crate::stats::{self, Stats};

const MAX_CONNECTIONS: usize = 256;
const TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) async fn run(
    listener: TcpListener,
    responses: Responses,
    stats: Stats,
) -> io::Result<()> {
    let server = Arc::new(Server {
        stats: Mutex::new(stats),
        responses,
    });
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        let (stream, _) = listener.accept().await?;
        let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
            continue;
        };
        let server = Arc::clone(&server);
        tokio::spawn(async move {
            let _slot = slot;
            let _ = timeout(TIMEOUT, server.handle(stream)).await;
        });
    }
}

struct Server {
    stats: Mutex<Stats>,
    responses: Responses,
}

impl Server {
    async fn handle(&self, stream: TcpStream) -> io::Result<()> {
        stream.set_nodelay(true)?;
        let mut reader = BufReader::with_capacity(MAX_PACKET, stream);
        let mut buf = [0; MAX_PACKET];
        let Some(hello) = protocol::handshake(protocol::read_packet(&mut reader, &mut buf).await?)
        else {
            return Ok(());
        };

        let packet = protocol::read_packet(&mut reader, &mut buf).await?;
        match hello.intent {
            Intent::Status if packet == [0] => {
                let count = self.stats.lock().expect("stats lock poisoned").count();
                let reply = self.responses.status(count, hello.protocol)?;
                reader.get_mut().write_all(&reply).await?;

                let packet = protocol::read_packet(&mut reader, &mut buf).await?;
                if packet.len() == 9 && packet[0] == 1 {
                    let mut pong = [0; 10];
                    pong[0] = 9;
                    pong[1..].copy_from_slice(packet);
                    reader.get_mut().write_all(&pong).await?;
                }
            }
            Intent::Login => {
                if let Some(player) = protocol::login_start(packet, hello.protocol) {
                    let count = self
                        .stats
                        .lock()
                        .expect("stats lock poisoned")
                        .add(player)
                        .inspect_err(|err| {
                            crate::log(
                                "stats_write_failed",
                                format_args!(
                                    "reason=append_failed file={} detail={:?}",
                                    stats::PATH,
                                    err.to_string()
                                ),
                            );
                        })?;
                    let reply = self.responses.disconnect(count, player)?;
                    reader.get_mut().write_all(&reply).await?;
                }
            }
            Intent::Status => {}
        }
        Ok(())
    }
}
