use ipnet::Ipv4Net;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::io::{Error, Read, Write};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

use crate::client::IP;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    addr: SocketAddr,
}

pub async fn serve() {
    info!("Starting Obirt");

    let networks = HashMap::<Ipv4Addr, Peer>::new();
    let mnetworks = Arc::new(RwLock::new(networks));

    let cnetworks = mnetworks.clone();

    let socket = UdpSocket::bind("0.0.0.0:1120").await.unwrap();
    let msocket = Arc::new(socket);
    info!("Listening for auth on 1120");

    tokio::spawn(auther(msocket.clone(), cnetworks));

    let cnetworks = mnetworks.clone();

    let socket = UdpSocket::bind("0.0.0.0:9807").await.unwrap();

    info!("Listening for packets on 9807");
    info!("Ready to route packets");

    tokio::join!(
        worker(0, &socket, cnetworks.clone()),
        worker(1, &socket, cnetworks.clone()),
        worker(2, &socket, cnetworks.clone()),
        worker(3, &socket, cnetworks.clone()),
        worker(4, &socket, cnetworks.clone()),
        worker(5, &socket, cnetworks.clone()),
        worker(6, &socket, cnetworks.clone()),
        worker(7, &socket, cnetworks.clone()),
    );
}

async fn auther(socket: Arc<UdpSocket>, networks: Arc<RwLock<HashMap<Ipv4Addr, Peer>>>) {
    let mut ip_to_mac = HashMap::<mac_address::MacAddress, Ipv4Addr>::new();

    let mut counter = 0;
    loop {
        let mut buffer = [0; 4096];
        let (read, addr) = match socket.recv_from(&mut buffer).await {
            Ok((read, addr)) => (read, addr),
            Err(_) => continue,
        };

        info!("New peer request from {}", addr);

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
            networks.insert(ip.clone(), Peer { addr });
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
            networks.insert(peer.addr(), Peer { addr });
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
    let mut data: Vec<u8> = vec![0; 1000000];
    let (read, addr) = match socket.recv_from(&mut data).await {
        Ok((read, addr)) => (read, addr),
        Err(e) => {
            return Err(e);
        }
    };

    return Ok(Received { read, addr, data });
}

async fn worker(id: u8, socket: &UdpSocket, networks: Arc<RwLock<HashMap<Ipv4Addr, Peer>>>) {
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

        let packet: IP = match bincode::deserialize(&data.data) {
            Ok(packet) => packet,
            Err(e) => {
                error!("Error deserializing packet");
                dbg!(e);
                continue;
            }
        };

        let source = packet.source;
        let destination = packet.destination;
        debug!("Packet is IP from {} to {}", source, destination);

        let networks = networks.read().await;

        let got = networks.get(&source);
        if let None = got {
            error!("Packet source is not in networks");
            dbg!(&source);

            continue;
        };

        let from = got.unwrap();

        if from.addr != data.addr {
            error!("Packet source does not match source address");
            dbg!(&from.addr);
            dbg!(&data.addr);

            continue;
        }

        let got = networks.get(&destination);
        if let None = got {
            error!("Packet destination is not in networks");
            dbg!(&destination);

            continue;
        };

        let to = got.unwrap();
        match socket
            .send_to(&packet.data[..packet.data.len()], to.addr)
            .await
        {
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
    }
}
