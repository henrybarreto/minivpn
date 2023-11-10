use bincode;
use ipnet::Ipv4Net;
use log::{debug, error, info, trace};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::{net::UdpSocket, sync::Mutex};
use tun::platform::posix::{Reader, Writer};

pub async fn connect(server: &str, port: &str, interface: &str) {
    info!("Obirt connecting to {}:{}", server, port);

    let socket = if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
        socket
    } else {
        panic!("Failed to bind");
    };

    info!("Registering peer on the server");

    info!("Generating key pair");
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RsaPublicKey::from(&priv_key);
    info!("Generated key pair");

    info!("Sending public key to server");
    socket
        .send_to(
            &bincode::serialize(&pub_key).unwrap(),
            format!("{}:{}", server, "1120"),
        )
        .await
        .unwrap();
    info!("Sent public key to server");

    info!("Waiting for server public key");

    let mut buf = [0; 4096];
    let (read, _) = socket.recv_from(&mut buf).await.unwrap();

    let bytes = bincode::deserialize(&buf[..read]);
    let pub_key: rsa::RsaPublicKey = match bytes {
        Ok(me) => me,
        Err(e) => {
            error!("Error deserializing server public key due {}", e);
            return;
        }
    };

    trace!("Received server public key");

    info!("Sending MAC address to server");
    let mac = mac_address::get_mac_address().unwrap();
    socket
        .send_to(
            &bincode::serialize(&mac.unwrap()).unwrap(),
            format!("{}:{}", server, "1120"),
        )
        .await
        .unwrap();

    info!("Sent MAC address to server");

    info!("Waiting for peer registration response");
    let (read, _) = socket.recv_from(&mut buf).await.unwrap();

    let bytes = bincode::deserialize(&buf[..read]);
    let peer: Ipv4Net = match bytes {
        Ok(me) => me,
        Err(e) => {
            error!("Error deserializing peer due {}", e);
            return;
        }
    };

    info!("Peer registered as {}", peer);

    let mut config = tun::Configuration::default();
    config
        .name(interface)
        .address(peer.addr())
        .netmask(peer.netmask())
        .queues(2)
        .up();

    let interface = tun::create(&config).unwrap();

    let (reader, writer) = interface.split();
    let reader = Arc::new(Mutex::new(reader));
    let writer = Arc::new(Mutex::new(writer));

    let router = format!("{}:{}", server, port);
    dbg!(&router);

    info!("Connecting to router");

    if let Err(e) = socket.connect(router).await {
        panic!("Failed to connect to router due {}", e);
    }

    info!("Connected to router");

    let sockets = Arc::new(socket);

    let socket = sockets.clone();
    let writer = writer.clone();
    tokio::spawn(async move {
        tokio::join!(
            input(0, socket.clone(), writer.clone(), priv_key.clone()),
            input(1, socket.clone(), writer.clone(), priv_key.clone()),
            input(2, socket.clone(), writer.clone(), priv_key.clone()),
            input(3, socket.clone(), writer.clone(), priv_key.clone())
        );
    });

    let socket = sockets.clone();
    let reader = reader.clone();
    tokio::spawn(async move {
        tokio::join!(
            output(0, socket.clone(), reader.clone(), pub_key.clone()),
            output(1, socket.clone(), reader.clone(), pub_key.clone()),
            output(2, socket.clone(), reader.clone(), pub_key.clone()),
            output(3, socket.clone(), reader.clone(), pub_key.clone())
        );
    });

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    let socket = sockets.clone();
    loop {
        trace!("Waiting for ping interval");
        interval.tick().await;
        trace!("Ping time interval reached");

        let buffer = [254; 1];
        if let Err(e) = socket.send(&buffer).await {
            error!("Failed to ping the server due {}", e);
        }

        info!("Ping");
    }
}
async fn input(
    id: usize,
    socket: Arc<UdpSocket>,
    interface: Arc<Mutex<Writer>>,
    private_key: RsaPrivateKey,
) {
    loop {
        trace!("Receiving cycle {}", id);

        let mut buffer: Vec<u8> = vec![0; 4096];

        let recved = socket.recv(&mut buffer).await;
        let read = match recved {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to receive packet due to {}", e);

                continue;
            }
        };

        info!("Received {} bytes using {}", read, id);

        let mut packet: Vec<u8> = Vec::new();
        let chunks = buffer[..read].chunks(256);
        for chunk in chunks {
            let mut p = match private_key.decrypt(Pkcs1v15Encrypt, &chunk[..chunk.len()]) {
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

        if let Ok(_ip) = packet::ip::v4::Packet::new(&packet[..packet.len()]) {
            let mut interface = interface.lock().await;
            let to_write = interface.write(&packet[..packet.len()]);
            if let Err(e) = to_write {
                error!("Failed to write packet due to {}", e);

                drop(interface);
                continue;
            }

            drop(interface);

            let wrote = to_write.unwrap();
            info!("Wrote {} bytes using {}", wrote, id);
        } else {
            info!("Packet read from socket is not IP");
        }
    }
}

async fn output(
    id: usize,
    socket: Arc<UdpSocket>,
    interface: Arc<Mutex<Reader>>,
    pub_key: rsa::RsaPublicKey,
) {
    loop {
        trace!("Sending cycle {}", id);

        let mut buffer: Vec<u8> = vec![0; 4096];

        let mut interface = interface.lock().await;
        let read = match interface.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to read packet due to {}", e);

                drop(interface);
                continue;
            }
        };

        info!("Read {} bytes using {}", read, id);

        // chuncks of 128 bytes
        if let Ok(_) = packet::ip::v4::Packet::new(&buffer[..read]) {
            debug!("Packet read from tun is IP");

            let mut data: Vec<u8> = Vec::new();

            let chunks = buffer[..read].chunks(128);
            for chunk in chunks {
                let mut rng = rand::thread_rng();
                let mut enc = pub_key
                    .encrypt(&mut rng, Pkcs1v15Encrypt, &chunk[..chunk.len()])
                    .unwrap();

                data.append(&mut enc);
            }

            let sent = match socket.send(&data[..data.len()]).await {
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
