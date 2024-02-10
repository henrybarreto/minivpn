use std::{
    collections::HashMap,
    io::Error,
    net::{Ipv4Addr, SocketAddr},
};

use openssl::{self, rsa::Padding};

use ipnet::Ipv4Net;
use log::{error, info, trace};
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

fn decrypt<'a>(data: &'a [u8], key: &'a [u8]) -> Result<Vec<u8>, rsa::Error> {
    let chipher = openssl::symm::Cipher::aes_128_ecb();

    let decrypted = openssl::symm::decrypt(chipher, key, None, &data).unwrap();

    return Ok(decrypted);
}

fn encrypt<'a>(data: &'a [u8], key: &'a [u8]) -> Result<Vec<u8>, rsa::Error> {
    let chipher = openssl::symm::Cipher::aes_128_ecb();

    let encrypted = openssl::symm::encrypt(chipher, key, None, &data).unwrap();

    return Ok(encrypted.to_owned());
}

async fn recv_enc<'a, T>(
    socket: &'a UdpSocket,
    buffer: &'a mut [u8],
    key: &'a [u8],
) -> Result<(T, SocketAddr), Error>
where
    T: serde::de::Deserialize<'a>,
{
    let (size, addr) = match socket.recv_from(buffer).await {
        Ok((read, addr)) => (read, addr),
        Err(_) => {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "failed to recv the buffer through the socket",
            ));
        }
    };

    let decrypted_value = decrypt(&buffer[..size], key).unwrap();
    let decrypted: &'a [u8] = decrypted_value.leak();

    let model: T = match bincode::deserialize::<'a>(decrypted) {
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

async fn send_dec<'a, T>(
    socket: &'a UdpSocket,
    addr: SocketAddr,
    model: &'a T,
    key: &'a [u8],
) -> Result<usize, Error>
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

    let encrypted_value = encrypt(&buffer[..buffer.len()], key).unwrap();
    let encrypted: &'a [u8] = encrypted_value.leak();

    let result = socket.send_to(&encrypted, addr).await;
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

    // let key = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";

    let mut table = HashMap::<mac_address::MacAddress, Ipv4Addr>::new();

    let server_private_key = openssl::rsa::Rsa::generate(2048).unwrap();
    let server_public_key = server_private_key.public_key_to_pem().unwrap();

    let aes_bytes = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F";
    // let aes_key = openssl::aes::AesKey::new_encrypt(aes_bytes).unwrap();

    let mut counter = 0;
    loop {
        let mut buffer = vec![0; 4096];

        trace!("receiving peer public key");

        let received = recv::<Vec<u8>>(&socket, &mut buffer).await;
        if let Err(_) = received {
            error!("Error receiving the public key");

            continue;
        }

        trace!("peer public key received");

        let (key, addr) = received.unwrap();

        let decode = openssl::rsa::Rsa::public_key_from_pem(&key);
        if decode.is_err() {
            error!("data present as RSA public key isn't valid");

            continue;
        }

        let peer_public_key = decode.unwrap();

        trace!("sending server public key");

        if let Err(_) = send(&socket, addr, &server_public_key).await {
            error!("failed to send the server public key");

            continue;
        }

        trace!("server public key sent");

        let mut encrypted_key = vec![0; 4096];
        let encrypted_key_size = peer_public_key
            .public_encrypt(aes_bytes, &mut encrypted_key, Padding::PKCS1)
            .unwrap();

        let to_send_encrypted_key = &encrypted_key[..encrypted_key_size];

        if let Err(_) = send(&socket, addr, &to_send_encrypted_key).await {
            error!("failed to send the AES key");

            continue;
        }

        // ---

        let received = recv_enc::<mac_address::MacAddress>(&socket, &mut buffer, aes_bytes).await;
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

            if let Err(_) = send_dec(&socket, addr, &peer, aes_bytes).await {
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

            if let Err(_) = send_dec(&socket, addr, &peer, aes_bytes).await {
                error!("failed to send the peer addres through the socket");

                continue;
            }

            info!("Added peer: {} as {}", addr, peer.addr());

            counter += 1;
        }
    }
}
