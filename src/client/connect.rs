use bincode;
use ipnet::Ipv4Net;
use log::{error, info, trace};
use rsa::{pkcs1::DecodeRsaPublicKey, RsaPublicKey};
use std::{collections::HashMap, fs, io::Read, net::Ipv4Addr, sync::Arc};
use tokio::{net::UdpSocket, sync::Mutex};

use crate::client::{io, loader};
pub async fn connect(server: &str, port: &str, interface: &str) {
    info!("Obirt connecting to {}:{}", server, port);

    let socket = if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
        socket
    } else {
        panic!("Failed to bind");
    };

    info!("Binded to {}", socket.local_addr().unwrap());

    trace!("Registering peer on the server");

    info!("Sending information to server");

    let mut buffer = [0; 4096];
    trace!("Sending MAC address to server");
    let mac = mac_address::get_mac_address().unwrap();
    socket
        .send_to(
            &bincode::serialize(&mac.unwrap()).unwrap(),
            format!("{}:{}", server, "1120"),
        )
        .await
        .unwrap();

    trace!("Sent MAC address to server");

    trace!("Waiting for peer registration response");
    let (read, _) = socket.recv_from(&mut buffer).await.unwrap();

    let bytes = bincode::deserialize(&buffer[..read]);
    let peer: Ipv4Net = match bytes {
        Ok(me) => me,
        Err(e) => {
            panic!("Error deserializing peer due {}", e);
        }
    };

    info!("Peer registered as {}", peer);

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

    let socket = sockets.clone();
    let writer = writer.clone();
    tokio::spawn(async move {
        tokio::join!(
            io::input(0, &socket, writer.clone(), &private),
            io::input(1, &socket, writer.clone(), &private),
            io::input(2, &socket, writer.clone(), &private),
            io::input(3, &socket, writer.clone(), &private)
        );
    });

    let socket = sockets.clone();
    let reader = reader.clone();
    tokio::spawn(async move {
        tokio::join!(
            io::output(0, socket.clone(), reader.clone(), &peers),
            io::output(1, socket.clone(), reader.clone(), &peers),
            io::output(2, socket.clone(), reader.clone(), &peers),
            io::output(3, socket.clone(), reader.clone(), &peers)
        );
    });

    let socket = sockets.clone();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
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
