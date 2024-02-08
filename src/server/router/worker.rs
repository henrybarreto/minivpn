use std::{io::Error, net::SocketAddr};

use tokio::net::UdpSocket;

use log::{debug, error, info, trace};

use crate::client::IP;
use crate::server::entities::Peers;

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

pub async fn recv(socket: &UdpSocket) -> Result<Received, Error> {
    let mut buffer: Vec<u8> = vec![0; 4096];
    let (read, addr) = match socket.recv_from(&mut buffer).await {
        Ok((read, addr)) => (read, addr),
        Err(e) => {
            return Err(e);
        }
    };

    return Ok(Received {
        read,
        addr,
        data: buffer,
    });
}

pub async fn send(socket: &UdpSocket, addr: SocketAddr, buffer: Vec<u8>) -> Result<Sent, Error> {
    let sent = socket.send_to(&buffer[..buffer.len()], addr).await;
    if let Err(e) = sent {
        error!("Error sending packet");
        dbg!(&e);

        return Err(e);
    }

    let wrote = sent.unwrap();

    return Ok(Sent {
        wrote,
        addr,
        data: buffer,
    });
}

/// init the worker to deals with IP packages, redirect it to the right peer.
pub async fn init(socket: &UdpSocket, peers: &impl Peers) {
    info!("worker started");

    loop {
        trace!("Packet router cycle",);

        trace!("Waiting for packet on worker");
        let received = recv(&socket).await;
        if let Err(e) = received {
            error!("Error receiving packet");
            dbg!(e);

            continue;
        }

        let data = received.unwrap();

        debug!("Packet from {} reading {} on worker", data.addr, data.read);

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

        let got = peers.get(source).await;
        if let None = got {
            error!("Peer source is not in networks");
            dbg!(&source);

            continue;
        };

        let from = got.unwrap();

        if from.addr != data.addr {
            // TODO: send message to restart the client.

            error!("Peer source does not match source address");
            dbg!(&from.addr);
            dbg!(&data.addr);

            continue;
        }

        let got = peers.get(destination).await;
        if let None = got {
            error!("Peer destination is not in networks");
            dbg!(&destination);

            continue;
        };

        let to = got.unwrap();
        let data = send(&socket, to.addr, packet.data).await;
        if let Err(_) = data {
            error!("Error sending packet");

            continue;
        }

        let sent = data.unwrap();
        debug!("Data sent to {} on worker", sent.addr);

        info!("Packet routed from {} to {}", source, destination);
    }
}
