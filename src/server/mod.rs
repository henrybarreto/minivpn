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

mod auther;

use crate::client::IP;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    addr: SocketAddr,
}

const AUTH_SERVER: &str = "0.0.0.0";
const AUTH_PORT: u16 = 1120;
const ROUTER_SERVER: &str = "0.0.0.0";
const ROUTER_PORT: u16 = 9807;

pub async fn serve() {
    info!("Starting Obirt");

    let networks = HashMap::<Ipv4Addr, Peer>::new();
    let mnetworks = Arc::new(RwLock::new(networks));

    let cnetworks = mnetworks.clone();

    let socket = UdpSocket::bind(format!("{}:{}", AUTH_SERVER, AUTH_PORT))
        .await
        .unwrap();
    let msocket = Arc::new(socket);
    info!("Listening for auth on 1120");

    tokio::spawn(auther::auther(msocket, cnetworks));

    let cnetworks = mnetworks.clone();

    let socket = UdpSocket::bind(format!("{}:{}", ROUTER_SERVER, ROUTER_PORT))
        .await
        .unwrap();
    let sockets = Arc::new(socket);

    info!("Listening for packets on 9807");
    info!("Ready to route packets");

    tokio::join!(
        worker(0, sockets.clone(), cnetworks.clone()),
        worker(1, sockets.clone(), cnetworks.clone()),
        worker(2, sockets.clone(), cnetworks.clone()),
        worker(3, sockets.clone(), cnetworks.clone()),
        worker(4, sockets.clone(), cnetworks.clone()),
        worker(5, sockets.clone(), cnetworks.clone()),
        worker(6, sockets.clone(), cnetworks.clone()),
        worker(7, sockets.clone(), cnetworks.clone()),
    );
}

#[derive(Debug, Clone)]
pub struct Received {
    pub read: usize,
    pub addr: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Sent {
    pub wrote: usize,
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

async fn send(socket: &UdpSocket, addr: SocketAddr, data: Vec<u8>) -> Result<Sent, Error> {
    let sent = socket.send_to(&data[..data.len()], addr).await;
    if let Err(e) = sent {
        error!("Error sending packet");
        dbg!(&e);

        return Err(e);
    }

    let wrote = sent.unwrap();

    return Ok(Sent { wrote, addr, data });
}

async fn worker(id: u8, socket: Arc<UdpSocket>, networks: Arc<RwLock<HashMap<Ipv4Addr, Peer>>>) {
    loop {
        trace!("Packet router cycle {}", id);

        trace!("Waiting for packet on worker {}", id);
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

        let socket = socket.clone();
        let networks = networks.clone();
        tokio::spawn(async move {
            let packet: IP = match bincode::deserialize(&data.data) {
                Ok(packet) => packet,
                Err(e) => {
                    error!("Error deserializing packet");
                    dbg!(e);

                    return;
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

                return;
            };

            let from = got.unwrap();

            if from.addr != data.addr {
                error!("Packet source does not match source address");
                dbg!(&from.addr);
                dbg!(&data.addr);

                return;
            }

            let got = networks.get(&destination);
            if let None = got {
                error!("Packet destination is not in networks");
                dbg!(&destination);

                return;
            };

            let to = got.unwrap();
            let data = send(&socket, to.addr, packet.data).await;
            if let Err(_) = data {
                error!("Error sending packet");
                return;
            }

            let sent = data.unwrap();
            debug!("Data sent to {} on worker {}", sent.addr, id);

            info!("Packet sent from {} to {}", source, destination);
        });
    }
}
