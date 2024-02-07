use std::{
    collections::HashMap,
    io::{Read, Write},
    net,
    sync::Arc,
};

use log::{debug, error, info, trace};
use tokio::{net::UdpSocket, sync::Mutex};
use tun::platform::posix::{Reader, Writer};

use crate::client::{data, IP};

pub async fn input(
    id: usize,
    socket: &UdpSocket,
    interface: Arc<Mutex<Writer>>,
    private_key: &rsa::RsaPrivateKey,
) {
    loop {
        trace!("Receiving cycle {}", id);

        let mut buffer = [0 as u8; 4096];

        let recved = socket.recv(&mut buffer).await;
        let read = match recved {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to receive packet due to {}", e);

                continue;
            }
        };

        info!("Received {} bytes using {}", read, id);

        let interface = interface.clone();
        let private_key = private_key.clone();
        tokio::spawn(async move {
            let packet = match data::decrypt(buffer[..read].to_vec(), &private_key) {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to decrypt packet due to {}", e);

                    return;
                }
            };

            if let Ok(_ip) = packet::ip::v4::Packet::new(&packet[..packet.len()]) {
                let mut interface = interface.lock().await;
                let to_write = interface.write(&packet[..packet.len()]);
                if let Err(e) = to_write {
                    error!("Failed to write packet due to {}", e);

                    drop(interface);
                    return;
                }
                drop(interface);

                let wrote = to_write.unwrap();
                info!("Wrote {} bytes using {}", wrote, id);
            } else {
                info!("Packet read from socket is not IP");
            }
        });
    }
}

pub async fn output(
    id: usize,
    socket: &UdpSocket,
    interface: Arc<Mutex<Reader>>,
    peers: &HashMap<net::Ipv4Addr, rsa::RsaPublicKey>,
) {
    let mut buffer = [0 as u8; 4096];

    loop {
        trace!("Sending cycle {}", id);

        let mut interface = interface.lock().await;
        let read = match interface.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to read packet due to {}", e);

                drop(interface);
                continue;
            }
        };
        drop(interface);

        info!("Read {} bytes using {}", read, id);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            let source: net::Ipv4Addr = ip.source();
            let destination: net::Ipv4Addr = ip.destination();
            debug!("Packet read from tun is IP");

            let got = peers.get(&destination);
            if let None = got {
                error!("Client does not have a public key to this peer");
                dbg!(&destination);

                continue;
            };

            let key = got.unwrap();
            let data = match data::encrypt(buffer[..read].to_vec(), key) {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to encrypt packet due to {}", e);

                    continue;
                }
            };

            let sent = match socket
                .send(
                    &bincode::serialize(&IP {
                        source,
                        destination,
                        data,
                    })
                    .unwrap(),
                )
                .await
            {
                Ok(sent) => {
                    dbg!(sent);
                    sent
                }
                Err(e) => {
                    error!("Failed to send packet due to {}", e);

                    continue;
                }
            };

            info!("Sent {} bytes using {}", sent, id);
        } else {
            debug!("Packet read from tun is not IP");
        }
    }
}
