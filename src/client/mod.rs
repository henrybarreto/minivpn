use bincode;
use ipnet::Ipv4Net;
use log::{error, info, trace};
use openssl::rsa::Padding;
use std::{
    io::{Error, Read},
    net::{Ipv4Addr, SocketAddr},
    process::exit,
    sync::{Arc, OnceLock},
};

use tokio::{net::UdpSocket, sync::Mutex, time};
use tun::platform::posix::{Reader, Writer};

use serde::{Deserialize, Serialize};

pub mod data;
pub mod io;

pub static AES_KEY: OnceLock<Vec<u8>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IP {
    pub source: Ipv4Addr,
    pub destination: Ipv4Addr,
    pub data: Vec<u8>,
}

async fn recv<'a, T>(
    socket: &'a UdpSocket,
    buffer: &'a mut [u8],
) -> Result<(usize, SocketAddr, T), Error>
where
    T: serde::de::Deserialize<'a>,
{
    let timeout =
        tokio::time::timeout(time::Duration::from_secs(5), socket.recv_from(buffer)).await;
    if timeout.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "failed to recv the buffer through the socket due timeout",
        ));
    }

    let (read, addr) = match timeout.unwrap() {
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

    return Ok((read, addr, model));
}

async fn send<'a, T>(socket: &'a UdpSocket, model: &'a T) -> Result<usize, Error>
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

    let result = socket.send(&buffer).await;
    if let Err(_) = result {
        return Err(Error::new(
            std::io::ErrorKind::Other,
            "failed to send the buffer through the socket",
        ));
    }

    let sent = result.unwrap();

    return Ok(sent);
}

struct Interface {
    reader: Arc<Mutex<Reader>>,
    writer: Arc<Mutex<Writer>>,
}

impl Interface {
    /// Creates a new network interface using name and peer as its address.
    pub fn new(name: &str, peer: Ipv4Net) -> Self {
        let mut config = tun::Configuration::default();
        config
            .name(name)
            .address(peer.addr())
            .netmask(peer.netmask())
            .up();

        let interface = tun::create(&config).unwrap();

        let (reader, writer) = interface.split();
        let reader = Arc::new(Mutex::new(reader));
        let writer = Arc::new(Mutex::new(writer));

        return Interface { reader, writer };
    }
}

struct Authenticator<'a> {
    socket: &'a UdpSocket,
    address: String,
    port: String,
}

impl<'a> Authenticator<'a> {
    pub fn new(socket: &'a UdpSocket, address: String, port: String) -> Self {
        return Authenticator {
            socket,
            address,
            port,
        };
    }

    pub async fn connect(&'a self) -> Result<bool, Error> {
        let auther = format!("{}:{}", self.address, self.port);
        dbg!(&auther);

        if let Err(_) = self.socket.connect(auther).await {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                "failed to connect to the authentication server",
            ));
        }

        return Ok(true);
    }

    pub async fn authenticate(&self) -> Result<Ipv4Net, Error> {
        trace!("Registering peer on the server");

        info!("Sending information to server");

        let mut buffer = vec![0; 4096];

        let socket = self.socket;

        let peer_private_key = openssl::rsa::Rsa::generate(2048).unwrap();
        let peer_public_key = peer_private_key.public_key_to_pem().unwrap();

        trace!("sending peer public key");

        if let Err(e) = send::<Vec<u8>>(&socket, &peer_public_key).await {
            error!("failed to send peer public key");

            return Err(e);
        }

        trace!("peer public key sent");

        trace!("receiving server public key");

        let received = recv::<Vec<u8>>(&socket, &mut buffer).await;
        if let Err(e) = received {
            error!("failed to receive the server public key ");

            return Err(e);
        }

        trace!("server public key received");

        let (_, _, key) = received.unwrap();

        let decode = openssl::rsa::Rsa::public_key_from_pem(&key);
        if let Err(_) = decode {
            error!("Data present as RSA public key isn't valid");

            return Err(Error::new(
                std::io::ErrorKind::Other,
                "Data present as RSA public key isn't valid",
            ));
        }

        let server_public_key = decode.unwrap();

        trace!("receiving aes key");

        let received = recv::<Vec<u8>>(&socket, &mut buffer).await;
        if let Err(e) = received {
            error!("failed to receive the server AES key ");

            return Err(e);
        }

        trace!("server aes key received");

        let (_, _, encrypted_key) = received.unwrap();

        let mut aes_bytes = vec![0; 4096];
        let aes_bytes_size = peer_private_key
            .private_decrypt(&encrypted_key, &mut aes_bytes, Padding::PKCS1)
            .unwrap();

        AES_KEY
            .set(Vec::from(&aes_bytes[..aes_bytes_size]))
            .unwrap();

        trace!("Sending MAC address to server");
        let mac = mac_address::get_mac_address().unwrap().unwrap();

        if let Err(e) = send::<mac_address::MacAddress>(&socket, &mac).await {
            error!("failed to send the MAC address: {}", e);

            return Err(e);
        }

        trace!("Sent MAC address to server");

        trace!("Waiting for peer registration response");
        let received = recv::<Ipv4Net>(&socket, &mut buffer).await;
        if let Err(e) = received {
            error!("failed to receive the peer address");

            return Err(e);
        }

        let (_, _, peer) = received.unwrap();

        return Ok(peer);
    }
}

pub async fn connect(server: &str, auth_port: &str, router_port: &str, interface: &str) {
    info!("Obirt connecting to {}", server);

    let socket = if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
        socket
    } else {
        panic!("Failed to bind");
    };

    info!("Binded to {}", socket.local_addr().unwrap());

    let authenticator = Authenticator::new(&socket, server.to_string(), auth_port.to_string());
    if let Err(e) = authenticator.connect().await {
        panic!("failed to connect to the authentication server: {}", e);
    }

    let peer: Ipv4Net;

    let authentication = authenticator.authenticate().await;
    if let Err(e) = authentication {
        panic!("failed to authenticate on the authentication server: {}", e);
    }

    peer = authentication.unwrap();

    info!("Peer registered as {}", peer);

    let interface = Interface::new(interface, peer);

    let router = format!("{}:{}", server, router_port);
    dbg!(&router);

    trace!("Connecting to router");

    if let Err(e) = socket.connect(router).await {
        panic!("Failed to connect to router due {}", e);
    }

    let sockets = Arc::new(socket);

    info!("Connected to router");

    let input = sockets.clone();
    let output = sockets.clone();

    tokio::spawn(async move { io::input(&input, interface.writer).await });
    tokio::spawn(async move { io::output(&output, interface.reader).await });

    let pinger = sockets.clone();

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        trace!("Waiting for ping interval");
        interval.tick().await;
        trace!("Ping time interval reached");

        let keepalive = [254; 1];
        if let Err(e) = pinger.send(&keepalive).await {
            error!("Failed to ping the server due {}", e);

            exit(1);
        }

        info!("Ping");
    }
}
