use std::{collections::HashMap, net::Ipv4Addr};

use ipnet::Ipv4Net;
use log::{error, info};
use tokio::net::UdpSocket;

use crate::server::entities::{Address, Peer, Peers};

const AUTH_SERVER: &str = "0.0.0.0";
const AUTH_PORT: u16 = 1120;

pub async fn start(peers: &impl Peers) {
    info!("Initing Obirt authenticator");

    let address = Address::new(AUTH_SERVER, AUTH_PORT);

    let socket = UdpSocket::bind(address.to_string()).await.unwrap();
    info!(
        "Obirt authenticator listening for packets on {}",
        address.port
    );

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

            peers.set(ip.clone(), Peer { addr }).await;

            info!("Sending peer address to {}", addr);
            socket
                .send_to(&bincode::serialize(&peer).unwrap(), addr)
                .await
                .unwrap();

            info!("Added peer: {} as {}", addr, ip);
        } else {
            info!("New peer to register");
            let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100 + counter), 24).unwrap();

            peers.set(peer.addr(), Peer { addr }).await;

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
