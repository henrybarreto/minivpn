use ipnet::Ipv4Net;
use log::{debug, error, info, trace, warn};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use std::io::{Error, Read, Write};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
struct Peer {
    addr: SocketAddr,
    key: RsaPublicKey,
}

pub async fn serve() {
    info!("Starting Obirt");

    info!("Generating RSA pair");
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RsaPublicKey::from(&priv_key);
    info!("RSA key pair generated");

    let networks = HashMap::<Ipv4Addr, Peer>::new();
    let mnetworks = Arc::new(RwLock::new(networks));

    let cnetworks = mnetworks.clone();

    // TODO:
    let socket = UdpSocket::bind("0.0.0.0:1120").await.unwrap();
    let msocket = Arc::new(socket);
    info!("Listening for auth on 1120");

    tokio::spawn(auther(msocket.clone(), cnetworks, pub_key));

    let cnetworks = mnetworks.clone();

    let socket = UdpSocket::bind("0.0.0.0:9807").await.unwrap();

    info!("Listening for packets on 9807");
    info!("Ready to route packets");

    tokio::join!(
        worker(0, &socket, cnetworks.clone(), &priv_key),
        worker(1, &socket, cnetworks.clone(), &priv_key),
        worker(2, &socket, cnetworks.clone(), &priv_key),
        worker(3, &socket, cnetworks.clone(), &priv_key),
        worker(4, &socket, cnetworks.clone(), &priv_key),
        worker(5, &socket, cnetworks.clone(), &priv_key),
        worker(6, &socket, cnetworks.clone(), &priv_key),
        worker(7, &socket, cnetworks.clone(), &priv_key),
    );
}

async fn auther(
    socket: Arc<UdpSocket>,
    networks: Arc<RwLock<HashMap<Ipv4Addr, Peer>>>,
    pub_key: RsaPublicKey,
) {
    let mut ip_to_mac = HashMap::<mac_address::MacAddress, Ipv4Addr>::new();

    let mut counter = 0;
    loop {
        let mut buffer = [0; 4096];

        let (read, addr) = match socket.recv_from(&mut buffer).await {
            Ok((read, addr)) => (read, addr),
            Err(_) => continue,
        };

        info!("New peer request from {}", addr);

        let peer_key = match bincode::deserialize::<RsaPublicKey>(&buffer[..read]) {
            Ok(key) => key,
            Err(_) => continue,
        };

        info!("Received public key from {}", addr);

        info!("Sending public key to {}", addr);
        socket
            .send_to(&bincode::serialize(&pub_key.clone()).unwrap(), addr)
            .await
            .unwrap();
        info!("Sent public key to {}", addr);

        info!("Waiting for MAC address from {}", addr);

        let (read, addr) = match socket.recv_from(&mut buffer).await {
            Ok((read, addr)) => (read, addr),
            Err(_) => continue,
        };

        let mac = match bincode::deserialize::<mac_address::MacAddress>(&buffer[..read]) {
            Ok(mac) => mac,
            Err(_) => {
                error!("Error deserializing MAC address");

                continue;
            }
        };

        info!("Received peer request from MAC {}", mac);

        if let Some(ip) = ip_to_mac.get(&mac) {
            info!("Peer already registered");
            let peer = Ipv4Net::new(ip.clone(), 24).unwrap();

            let mut networks = networks.write().await;
            networks.insert(
                ip.clone(),
                Peer {
                    addr,
                    key: peer_key,
                },
            );
            drop(networks);

            info!("Sending peer address to {}", addr);
            socket
                .send_to(&bincode::serialize(&peer).unwrap(), addr)
                .await
                .unwrap();

            info!("Added peer: {} as {}", addr, ip);
        } else {
            info!("New peer to register");
            let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100 + counter), 24).unwrap();

            let mut networks = networks.write().await;
            networks.insert(
                peer.addr(),
                Peer {
                    addr,
                    key: peer_key,
                },
            );
            drop(networks);

            ip_to_mac.insert(mac, peer.addr());

            info!("Sending peer address to {}", addr);
            socket
                .send_to(&bincode::serialize(&peer).unwrap(), addr)
                .await
                .unwrap();

            info!("Added peer: {} as {}", addr, peer.addr());

            counter += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Received {
    pub read: usize,
    pub addr: SocketAddr,
    pub data: Vec<u8>,
}

async fn recv(socket: &UdpSocket) -> Result<Received, Error> {
    let mut data: Vec<u8> = vec![0; 4096];
    let (read, addr) = match socket.recv_from(&mut data).await {
        Ok((read, addr)) => (read, addr),
        Err(e) => {
            return Err(e);
        }
    };

    return Ok(Received { read, addr, data });
}

fn decrypt(data: Vec<u8>, priv_key: &RsaPrivateKey) -> Result<Vec<u8>, rsa::Error> {
    let mut packet: Vec<u8> = Vec::new();
    let chunks = data[..data.len()].chunks(256);
    for chunk in chunks {
        // TODO: bottleneck.
        let mut p = match priv_key.decrypt(Pkcs1v15Encrypt, &chunk[..chunk.len()]) {
            Ok(e) => e,
            Err(_) => {
                continue;
            }
        };

        packet.append(&mut p);
    }

    return Ok(packet);
}

fn encrypt(data: Vec<u8>, pub_key: &RsaPublicKey) -> Result<Vec<u8>, rsa::Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let chunks = data[..data.len()].chunks(128);
    for chunk in chunks {
        let mut rng = rand::thread_rng();
        let enc = pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &chunk[..chunk.len()]);
        if let Err(e) = enc {
            return Err(e);
        }

        buffer.append(&mut enc.unwrap());
    }

    return Ok(buffer);
}

async fn worker(
    id: u8,
    socket: &UdpSocket,
    networks: Arc<RwLock<HashMap<Ipv4Addr, Peer>>>,
    priv_key: &RsaPrivateKey,
) {
    loop {
        trace!("Packet router cycle {}", id);

        let buffer = recv(&socket).await;
        if let Err(_) = buffer {
            error!("Error receiving packet");
            continue;
        }

        let data = buffer.unwrap();

        debug!(
            "Packet from {} reading {} on worker {}",
            data.addr, data.read, id
        );

        let mut packet: Vec<u8> = Vec::new();
        let chunks = data.data[..data.read].chunks(256);
        for chunk in chunks {
            // TODO: bottleneck.
            let mut p = match priv_key.decrypt(Pkcs1v15Encrypt, &chunk[..chunk.len()]) {
                Ok(e) => e,
                Err(e) => {
                    error!("Error decrypting packet");
                    dbg!(e);

                    continue;
                }
            };

            packet.append(&mut p);
        }

        // let decrypted = decrypt(buffer, priv_key);
        // if let Err(_) = decrypted {
        //     continue;
        // }

        // let packet = decrypted.unwrap();

        trace!("Packet router spawn to dial {}", data.addr);
        if let Ok(ip) = packet::ip::v4::Packet::new(&packet[..packet.len()]) {
            let source: Ipv4Addr = ip.source();
            let destination: Ipv4Addr = ip.destination();
            info!("Packet is IP from {} to {}", source, destination);

            let networks = networks.read().await;

            // let from = match networks.get(&source) {
            //     Some(from) => from,
            //     None => {
            //         error!("Packet source is not in networks");
            //         dbg!(&source);

            //         continue;
            //     }
            // };

            let got = networks.get(&destination);
            if let None = got {
                error!("Packet destination is not in networks");
                dbg!(&destination);

                continue;
            };

            let to = got.unwrap();

            let encrypted = encrypt(packet, &to.key);
            if let Err(e) = encrypted {
                error!("Failed to encrypt the IP packet for {}", &destination);

                dbg!(e);
                continue;
            }

            let data = encrypted.unwrap();

            match socket.send_to(&data[..data.len()], to.addr).await {
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
            warn!("Packet received from {} is not IP", data.addr);
        }
    }
}
