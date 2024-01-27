use std::{
    collections::HashMap,
    io::Error,
    net::{Ipv4Addr, SocketAddr},
};

use ipnet::Ipv4Net;
use log::{error, info};
use tokio::net::UdpSocket;

use crate::server::entities::{Address, Peer, Peers};

const AUTH_SERVER: &str = "0.0.0.0";
const AUTH_PORT: u16 = 1120;

async fn recv<'a, T>(socket: &'a UdpSocket, buffer: &'a mut [u8]) -> Result<(T, SocketAddr), Error>
where
    T: serde::de::Deserialize<'a>,
{
    let (_, addr) = match socket.recv_from(buffer).await {
        Ok((read, addr)) => (read, addr),
        Err(_) => {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "failed to recv the buffer through the socket",
            ));
        }
    };

    let model: T = match bincode::deserialize::<'a>(buffer) {
        Ok(mac) => mac,
        Err(_) => {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "failed to deserialize the data received",
            ))
        }
    };

    return Ok((model, addr));
}

async fn send<'a, T>(socket: &'a UdpSocket, addr: SocketAddr, model: &'a T) -> Result<usize, Error>
where
    T: serde::ser::Serialize,
{
    let ser = bincode::serialize(&model);
    if let Err(_) = ser {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "failed to serialize the data to send",
        ));
    }

    let buffer = ser.unwrap();

    let result = socket.send_to(&buffer, addr).await;
    if let Err(_) = result {
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "failed to send the buffer through the socket",
        ));
    }

    let sent = result.unwrap();

    return Ok(sent);
}

pub async fn start(peers: &impl Peers) {
    info!("Initing Obirt authenticator");

    let address = Address::new(AUTH_SERVER, AUTH_PORT);

    let socket = UdpSocket::bind(address.to_string()).await.unwrap();
    info!(
        "Obirt authenticator listening for packets on {}",
        address.port
    );

    let mut table = HashMap::<mac_address::MacAddress, Ipv4Addr>::new();

    let mut counter = 0;
    loop {
        let mut buffer = [0; 4096];

        let received = recv::<mac_address::MacAddress>(&socket, &mut buffer).await;
        if let Err(_) = received {
            error!("Error deserializing MAC address");

            continue;
        }

        let (mac, addr) = received.unwrap();

        info!("Received peer request from MAC {}", mac);

        if let Some(ip) = table.get(&mac) {
            info!("Peer already registered");
            let peer = Ipv4Net::new(ip.clone(), 24).unwrap();

            peers.set(ip.clone(), Peer::new(addr)).await;

            info!("Sending peer address to {}", addr);

            if let Err(_) = send(&socket, addr, &peer).await {
                error!("failed to send the peer addres through the socket");

                continue;
            }

            info!("Added peer: {} as {}", addr, ip);
        } else {
            info!("New peer to register");
            let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100 + counter), 24).unwrap();

            peers.set(peer.addr(), Peer::new(addr)).await;

            table.insert(mac, peer.addr());

            info!("Sending peer address to {}", addr);

            if let Err(_) = send(&socket, addr, &peer).await {
                error!("failed to send the peer addres through the socket");

                continue;
            }

            info!("Added peer: {} as {}", addr, peer.addr());

            counter += 1;
        }
    }
}
