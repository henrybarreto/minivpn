use std::{
    io::{Read, Write},
    net,
    sync::Arc,
};

use log::{debug, error, info, trace};
use tokio::{net::UdpSocket, sync::Mutex};
use tun::platform::posix::{Reader, Writer};

use crate::client::{data, IP};

use super::AES_KEY;

pub async fn input(socket: &UdpSocket, interface: Arc<Mutex<Writer>>) {
    let dec_key = AES_KEY.get().unwrap();

    loop {
        trace!("Receiving cycle");

        let mut buffer = vec![0 as u8; 4096];

        let recved = socket.recv(&mut buffer).await;
        let read = match recved {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to receive packet due to {}", e);

                continue;
            }
        };

        info!("Received {} bytes", read);

        let buffer = data::decrypt(Vec::from(&buffer[..read]), &dec_key).unwrap();

        let interface = interface.clone();

        if let Ok(_ip) = packet::ip::v4::Packet::new(&buffer[..buffer.len()]) {
            let mut interface = interface.lock().await;
            let to_write = interface.write(&buffer[..buffer.len()]);
            if let Err(e) = to_write {
                error!("Failed to write packet due to {}", e);

                drop(interface);
                return;
            }
            drop(interface);

            let wrote = to_write.unwrap();
            info!("Wrote {} bytes", wrote);
        } else {
            info!("Packet read from socket is not IP");
        }
    }
}

pub async fn output(socket: &UdpSocket, interface: Arc<Mutex<Reader>>) {
    let enc_key = AES_KEY.get().unwrap();

    let mut buffer = [0 as u8; 4096];

    loop {
        trace!("Sending cycle");

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

        info!("Read {} bytes", read);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            let source: net::Ipv4Addr = ip.source();
            let destination: net::Ipv4Addr = ip.destination();
            debug!("Packet read from tun is IP");

            let sent = match socket
                .send(
                    &bincode::serialize(&IP {
                        source,
                        destination,
                        data: data::encrypt(Vec::from(&buffer[..read]), &enc_key).unwrap(),
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

            info!("Sent {} bytes", sent,);
        } else {
            debug!("Packet read from tun is not IP");
        }
    }
}
