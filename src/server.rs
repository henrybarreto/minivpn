use ipnet::Ipv4Net;
use log::{debug, error, info, trace, warn};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use std::io::{Read, Write};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

pub async fn serve() {
    info!("Starting Obirt");

    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RsaPublicKey::from(&priv_key);

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
            let mut buffer = [0; 1024];

            let (read, addr) = match socket.recv_from(&mut buffer).await {
                Ok((read, addr)) => (read, addr),
                Err(_) => continue,
            };

            let mac = match bincode::deserialize::<mac_address::MacAddress>(&buffer[..read]) {
                Ok(mac) => mac,
                Err(_) => continue,
            };

            info!("Received peer request from {} with MAC {}", addr, mac);

            info!("Sending public key to {}", addr);
            socket
                .send_to(&bincode::serialize(&pub_key.clone()).unwrap(), addr)
                .await
                .unwrap();

            // ---

            let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100 + counter), 24).unwrap();
            info!("Sending peer address to {}", addr);

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
    info!("Ready to route packets");

    let csocket = msocket.clone();

    tokio::join!(
        worker(0, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(1, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(2, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(3, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(4, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(5, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(6, csocket.clone(), cnetworks.clone(), priv_key.clone()),
        worker(7, csocket.clone(), cnetworks.clone(), priv_key.clone()),
    );
}

async fn worker(
    id: u8,
    csocket: Arc<UdpSocket>,
    cnetworks: Arc<RwLock<HashMap<Ipv4Addr, SocketAddr>>>,
    priv_key: RsaPrivateKey,
) {
    loop {
        trace!("Packet router cycle {}", id);

        let mut buffer: Vec<u8> = vec![0; 4096];

        let socket = csocket.clone();
        let (read, addr) = match socket.recv_from(&mut buffer).await {
            Ok((read, addr)) => (read, addr),
            Err(e) => {
                error!("Error receiving packet");
                dbg!(e);

                continue;
            }
        };

        debug!("Packet from {} reading {} on worker {}", addr, read, id);

        let mut packet: Vec<u8> = Vec::new();
        let chunks = buffer[..read].chunks(256);
        for chunk in chunks {
            let mut p = match priv_key.decrypt(Pkcs1v15Encrypt, &chunk[..chunk.len()]) {
                Ok(e) => e,
                Err(e) => {
                    error!("Error decrypting packet");
                    // dbg!(&buffer[..read]);
                    // dbg!(read);
                    dbg!(e);

                    continue;
                }
            };

            packet.append(&mut p);
        }

        let csocket = csocket.clone();
        let cnetworks = cnetworks.clone();

        trace!("Packet router spawn to dial {}", addr);
        let socket = csocket.clone();

        if let Ok(ip) = packet::ip::v4::Packet::new(&packet[..packet.len()]) {
            dbg!(&ip);
            let source: Ipv4Addr = ip.source();
            let destination: Ipv4Addr = ip.destination();
            debug!("Packet is IP from {} to {}", source, destination);

            trace!("Lock for reading networks");
            let networks = cnetworks.read().await;
            // dbg!(&networks);

            let m = networks.clone();

            drop(networks);
            trace!("Done reading networks locking");

            let w_from = m.get(&source);
            if w_from.is_none() {
                error!("Packet source is not in networks");
                // dbg!(&source);

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
            // dbg!(&to);
            match socket.send_to(&packet[..packet.len()], to).await {
                Ok(send) => {
                    if send == 0 {
                        error!("Nothing was sent");
                        warn!("Removing {} from networks", &destination);
                        //networks.remove(&destination);
                    }

                    info!(
                        "Sent {} bytes from {} to {} on worker {}",
                        send, source, destination, id
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
