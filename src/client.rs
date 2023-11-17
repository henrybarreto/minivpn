use bincode;
use ipnet::Ipv4Net;
use log::{debug, error, info, trace};
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPublicKey};
use rsa::pkcs8::LineEnding;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
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

    let mut buf = [0; 4096];
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

    let mut file = File::open("./peers.toml").await.unwrap();
    let mut buffer = [0; 4096];
    file.read(&mut buffer).await.unwrap();

    let peers_str: HashMap<Ipv4Addr, String> = toml::de::from_str(
        std::str::from_utf8(&buffer[..])
            .unwrap()
            .trim_matches(char::from(0)),
    )
    .unwrap();

    let mut file = File::open("./peer.toml").await.unwrap();
    let mut buffer = [0; 4096];
    file.read(&mut buffer).await.unwrap();

    let peer_str = std::str::from_utf8(&buffer[..])
        .unwrap()
        .trim_matches(char::from(0));

    let peer = rsa::RsaPrivateKey::from_pkcs1_pem(&peer_str).unwrap();

    let mut peers = HashMap::<Ipv4Addr, rsa::RsaPublicKey>::new();
    for p in peers_str {
        let str = p.1;
        let key = rsa::RsaPublicKey::from_pkcs1_pem(&str).unwrap();

        peers.insert(p.0, key);
    }

    let mpeers = Arc::new(RwLock::new(peers));

    let sockets = Arc::new(socket);

    let socket = sockets.clone();
    let writer = writer.clone();
    tokio::spawn(async move {
        tokio::join!(
            input(0, socket.clone(), writer.clone(), &peer),
            input(1, socket.clone(), writer.clone(), &peer),
            input(2, socket.clone(), writer.clone(), &peer),
            input(3, socket.clone(), writer.clone(), &peer)
        );
    });

    let socket = sockets.clone();
    let reader = reader.clone();
    tokio::spawn(async move {
        tokio::join!(
            output(0, socket.clone(), reader.clone(), mpeers.clone()),
            output(1, socket.clone(), reader.clone(), mpeers.clone()),
            output(2, socket.clone(), reader.clone(), mpeers.clone()),
            output(3, socket.clone(), reader.clone(), mpeers.clone())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IP {
    pub source: Ipv4Addr,
    pub destination: Ipv4Addr,
    pub data: Vec<u8>,
}

async fn input(
    id: usize,
    socket: Arc<UdpSocket>,
    interface: Arc<Mutex<Writer>>,
    private_key: &RsaPrivateKey,
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

        let packet = match decrypt(buffer[..read].to_vec(), private_key) {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to decrypt packet due to {}", e);

                continue;
            }
        };

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
    peers: Arc<RwLock<HashMap<Ipv4Addr, rsa::RsaPublicKey>>>,
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
        drop(interface);

        info!("Read {} bytes using {}", read, id);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            let source: Ipv4Addr = ip.source();
            let destination: Ipv4Addr = ip.destination();
            debug!("Packet read from tun is IP");

            let cloned = peers.clone();
            let peers = cloned.read().await;
            let got = peers.get(&destination);
            if let None = got {
                error!("Client does not have a public key to this peer");
                dbg!(&destination);

                continue;
            };

            let key = got.unwrap();
            let data = match encrypt(buffer[..read].to_vec(), key) {
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
