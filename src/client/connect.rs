use bincode;
use ipnet::Ipv4Net;
use log::{error, info, trace};
use rsa::{pkcs1::DecodeRsaPublicKey, RsaPublicKey};
use std::{
    collections::HashMap,
    fs,
    io::{Error, Read},
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{net::UdpSocket, sync::Mutex};
use tun::platform::posix::{Reader, Writer};

use crate::client::{io, loader};

async fn recv<'a, T>(
    socket: &'a UdpSocket,
    buffer: &'a mut [u8],
) -> Result<(usize, SocketAddr, T), Error>
where
    T: serde::de::Deserialize<'a>,
{
    let (read, addr) = match socket.recv_from(buffer).await {
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
pub async fn try_auth(socket: &UdpSocket) -> Result<Ipv4Net, Error> {
    trace!("Registering peer on the server");

    info!("Sending information to server");

    let mut buffer = [0; 4096];

    trace!("Sending MAC address to server");
    let mac = mac_address::get_mac_address().unwrap().unwrap();

    if let Err(e) = send::<mac_address::MacAddress>(socket, &mac).await {
        error!("failed to send the MAC address: {}", e);

        return Err(e);
    }

    trace!("Sent MAC address to server");

    trace!("Waiting for peer registration response");
    let received = recv::<Ipv4Net>(socket, &mut buffer).await;
    if let Err(e) = received {
        error!("failed to receive the peer address");

        return Err(e);
    }

    let (_, _, peer) = received.unwrap();

    return Ok(peer);
}

pub async fn create_interface(
    interface: &str,
    peer: Ipv4Net,
) -> (Arc<Mutex<Reader>>, Arc<Mutex<Writer>>) {
    let mut config = tun::Configuration::default();
    config
        .name(interface)
        .address(peer.addr())
        .netmask(peer.netmask())
        .up();

    let interface = tun::create(&config).unwrap();

    let (reader, writer) = interface.split();
    let reader = Arc::new(Mutex::new(reader));
    let writer = Arc::new(Mutex::new(writer));

    return (reader, writer);
}

pub async fn connect(server: &str, port: &str, interface: &str) {
    info!("Obirt connecting to {}:{}", server, port);

    let socket = if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
        socket
    } else {
        panic!("Failed to bind");
    };

    info!("Binded to {}", socket.local_addr().unwrap());

    let auther = format!("{}:{}", server, port);
    dbg!(&auther);

    if let Err(e) = socket.connect(auther).await {
        panic!("Failed to connect to router due {}", e);
    }

    let t = try_auth(&socket).await;
    if let Err(e) = t {
        panic!("failed to auth the client: {}", e);
    }

    let peer = t.unwrap();

    info!("Peer registered as {}", peer);

    let (reader, writer) = create_interface(interface, peer).await;

    let router = format!("{}:{}", server, port);
    dbg!(&router);

    trace!("Connecting to router");

    if let Err(e) = socket.connect(router).await {
        panic!("Failed to connect to router due {}", e);
    }

    let sockets = Arc::new(socket);

    info!("Connected to router");

    let mut peers: HashMap<Ipv4Addr, RsaPublicKey> = loader::peers().await;

    let mut buffer = [0; 4096];
    let mut file = fs::File::open("./public.txt").unwrap();
    file.read(&mut buffer).unwrap();

    let public_str = std::str::from_utf8(&buffer[..])
        .unwrap()
        .trim_matches(char::from(0));

    let public = rsa::RsaPublicKey::from_pkcs1_pem(&public_str).unwrap();
    let private = loader::private().await;

    info!("Private and public keys loaded");

    peers.insert(peer.addr(), public);

    let inputer = sockets.clone();

    let writer = writer.clone();
    tokio::spawn(async move {
        tokio::join!(
            io::input(0, &inputer, writer.clone(), &private),
            io::input(1, &inputer, writer.clone(), &private),
            io::input(2, &inputer, writer.clone(), &private),
            io::input(3, &inputer, writer.clone(), &private)
        );
    });

    let outputer = sockets.clone();

    let reader = reader.clone();
    tokio::spawn(async move {
        tokio::join!(
            io::output(0, &outputer, reader.clone(), &peers),
            io::output(1, &outputer, reader.clone(), &peers),
            io::output(2, &outputer, reader.clone(), &peers),
            io::output(3, &outputer, reader.clone(), &peers)
        );
    });

    let sss = sockets.clone();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        trace!("Waiting for ping interval");
        interval.tick().await;
        trace!("Ping time interval reached");

        let buffer = [254; 1];
        if let Err(e) = sss.send(&buffer).await {
            error!("Failed to ping the server due {}", e);
        }

        info!("Ping");
    }
}
