use ipnet::Ipv4Net;
use log::{debug, error, info, trace, warn};
use std::io::{Read, Write};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

pub async fn serve() {
    info!("Starting Orbit");

    let networks = HashMap::<Ipv4Addr, SocketAddr>::new();
    let mnetworks = Arc::new(RwLock::new(networks));

    // let net = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap();

    let cnetworks = mnetworks.clone();
    trace!("Spawning peer listener");
    tokio::spawn(async move {
        info!("Listening for peers on 1120");

        let mut counter = 0;

        let socket = UdpSocket::bind("0.0.0.0:1120").await.unwrap();
        loop {
            let mut buffer = [0; 1];

            let (_, addr) = match socket.recv_from(&mut buffer).await {
                Ok((read, addr)) => (read, addr),
                Err(_) => continue,
            };

            info!("Received peer request from {}", addr);

            let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100 + counter), 24).unwrap();
            socket
                .send_to(&bincode::serialize(&peer).unwrap(), addr)
                .await
                .unwrap();

            let mut networks = cnetworks.write().await;
            networks.insert(peer.addr(), addr);
            drop(networks);

            info!("Added peer: {} as {}", addr, peer.addr());

            counter += 1;
        }
    });

    let cnetworks = mnetworks.clone();

    let socket = UdpSocket::bind("0.0.0.0:9807").await.unwrap();
    let msocket = Arc::new(socket);

    info!("Listening for packets on 9807");

    let csocket = msocket.clone();

    tokio::join!(
        worker(0, csocket.clone(), cnetworks.clone()),
        worker(1, csocket.clone(), cnetworks.clone()),
        worker(2, csocket.clone(), cnetworks.clone()),
        worker(3, csocket.clone(), cnetworks.clone()),
        worker(4, csocket.clone(), cnetworks.clone()),
        worker(5, csocket.clone(), cnetworks.clone()),
        worker(6, csocket.clone(), cnetworks.clone()),
        worker(7, csocket.clone(), cnetworks.clone()),
    );
}

async fn worker(
    id: u8,
    csocket: Arc<UdpSocket>,
    cnetworks: Arc<RwLock<HashMap<Ipv4Addr, SocketAddr>>>,
) {
    loop {
        trace!("Packet router cycle {}", id);

        let mut buffer = [0; 4096];

        let socket = csocket.clone();
        let (read, addr) = match socket.recv_from(&mut buffer).await {
            Ok((read, addr)) => (read, addr),
            Err(e) => {
                error!("Error receiving packet");
                dbg!(e);

                continue;
            }
        };

        info!("Packet from {} reading {} on worker {}", addr, read, id);

        let csocket = csocket.clone();
        let cnetworks = cnetworks.clone();

        trace!("Packet router spawn to dial {}", addr);
        let socket = csocket.clone();

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
            let source: Ipv4Addr = ip.source();
            let destination: Ipv4Addr = ip.destination();
            debug!("Packet is IP from {} to {}", source, destination);

            trace!("Lock for reading networks");
            let networks = cnetworks.read().await;
            dbg!(&networks);

            let m = networks.clone();

            drop(networks);
            trace!("Done reading networks locking");

            let w_from = m.get(&source);
            if w_from.is_none() {
                error!("Packet source is not in networks");
                dbg!(&source);

                continue;
            }

            debug!("Packet source is in networks {}", &source);

            let from = w_from.unwrap();
            if from.to_string() != addr.to_string() {
                error!("Packet source is not from the same address");
                dbg!(from, &addr);

                continue;
            }

            debug!("Packet source is from the same address {}", &addr);

            let w_to = m.get(&destination);
            if w_to.is_none() {
                error!("Packet destination is not in networks");
                dbg!(&destination);

                continue;
            }

            debug!("Packet destination is in networks {}", &destination);

            let to = w_to.unwrap();
            dbg!(&to);
            match socket.send_to(&buffer[..read], to).await {
                Ok(send) => {
                    if send == 0 {
                        error!("Nothing was sent");
                        warn!("Removing {} from networks", &destination);
                        //networks.remove(&destination);
                    }

                    info!(
                        "Sent {} bytes from {} to {} on worker {}",
                        send, from, to, id
                    );
                }
                Err(e) => {
                    error!("Error sending packet");
                    dbg!(e);
                    // When the destination is not reachable, remove it from the list.
                    //networks.remove(&destination);
                }
            }
        } else {
            warn!("Packet received from {} is not IP", addr);
        }
    }
}
