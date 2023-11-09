use bincode;
use ipnet::Ipv4Net;
use log::{debug, error, info, trace};
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

    // TODO: send a identity to server identify the peer and bind it to a address.
    // TODO: validate errors.
    socket
        .send_to(
            &bincode::serialize(&[0; 1]).unwrap(),
            format!("{}:{}", server, "1120"),
        )
        .await
        .unwrap();

    debug!("Waiting for server to respond");

    // TODO: validate errors.
    let mut buf = [0; 1024];
    let (read, _) = socket.recv_from(&mut buf).await.unwrap();

    info!("Received response from server {}", read);

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

    let msocket = Arc::new(socket);

    let cwriter = writer.clone();
    let csocket = msocket.clone();
    tokio::spawn(async move {
        tokio::join!(
            input(0, cwriter.clone(), csocket.clone()),
            input(1, cwriter.clone(), csocket.clone()),
            input(2, cwriter.clone(), csocket.clone()),
            input(3, cwriter.clone(), csocket.clone())
        );
    });

    let creader = reader.clone();
    let csocket = msocket.clone();
    tokio::spawn(async move {
        tokio::join!(
            output(0, creader.clone(), csocket.clone()),
            output(1, creader.clone(), csocket.clone()),
            output(2, creader.clone(), csocket.clone()),
            output(3, creader.clone(), csocket.clone())
        );
    });

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

    let socket = msocket.clone();
    loop {
        trace!("Waiting for ping interval");
        interval.tick().await;
        trace!("Ping time interval reached");

        let buffer = [0; 1];
        if let Err(e) = socket.send(&buffer).await {
            error!("Failed to ping the server due {}", e);
        }

        info!("Ping");
    }
}
async fn input(id: usize, cwriter: Arc<Mutex<Writer>>, csocket: Arc<UdpSocket>) {
    loop {
        trace!("Receiving cycle {}", id);

        let mut buffer = [0; 4096];

        let socket = csocket.clone();
        let recved = socket.recv(&mut buffer).await;
        let read = match recved {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to receive packet due to {}", e);

                continue;
            }
        };

        let mut interface = cwriter.lock().await;
        let to_write = interface.write(&buffer[..read]);
        if let Err(e) = to_write {
            error!("Failed to write packet due to {}", e);

            drop(interface);
            continue;
        }

        drop(interface);

        let wrote = to_write.unwrap();
        info!("Wrote {} bytes using {}", wrote, id);
    }
}

async fn output(id: usize, creader: Arc<Mutex<Reader>>, csocket: Arc<UdpSocket>) {
    loop {
        trace!("Sending cycle {}", id);

        let mut buffer = [0; 4096];

        let mut interface = creader.lock().await;
        let read = match interface.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                error!("Failed to read packet due to {}", e);

                drop(interface);
                continue;
            }
        };

        if let Ok(_) = packet::ip::v4::Packet::new(&buffer[..read]) {
            debug!("Packet read from tun is IP");

            let socket = csocket.clone();
            let sent = match socket.send(&buffer[..read]).await {
                Ok(read) => read,
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
