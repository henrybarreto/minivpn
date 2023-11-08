use bincode;
use ipnet::Ipv4Net;
use log::{debug, error, info, trace};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::{net::UdpSocket, sync::Mutex};

pub async fn connect(server: &str, port: &str, interface: &str) {
    info!("Obirt connecting to {}:{}", server, port);

    let mut config = tun::Configuration::default();

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("couldn't bind to address");

    info!("Bound to 1620");

    // let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100), 24).unwrap();
    info!("Registering perr on the server");

    socket
        .send_to(
            &bincode::serialize(&[0; 1]).unwrap(),
            format!("{}:{}", server, "1120"),
        )
        .await
        .unwrap();

    debug!("Waiting for server to respond");

    let mut buf = [0; 1024];
    let (read, _) = socket.recv_from(&mut buf).await.unwrap();

    info!("Received response from server {}", read);

    let to_peer = bincode::deserialize(&buf[..read]);
    let peer: Ipv4Net = match to_peer {
        Ok(me) => me,
        Err(e) => {
            error!("Error deserializing peer due {}", e);
            return;
        }
    };

    info!("Peer registered as {}", peer);

    config.address(peer.addr()).netmask(peer.netmask()).up();
    config.queues(2);
    config.name(interface);

    let dev = tun::create(&config).unwrap();
    let (reader, writer) = dev.split();

    let router = format!("{}:{}", server, port);
    dbg!(&router);

    info!("Connecting to router");

    socket
        .connect(router)
        .await
        .expect("couldn't connect to address");

    info!("Connected to router");

    let reader = Arc::new(Mutex::new(reader));
    let writer = Arc::new(Mutex::new(writer));

    let msocket = Arc::new(socket);

    let cwriter = writer.clone();
    let csocket = msocket.clone();
    tokio::spawn(async move {
        loop {
            trace!("Receiving cycle");

            let mut buffer = [0; 4096];

            let socket = csocket.clone();

            let recved = socket.recv(&mut buffer).await;
            if let Err(e) = recved {
                error!("Failed to receive packet due to {}", e);

                continue;
            }

            let read = recved.unwrap();
            info!("Received {} bytes", read);

            let mut dev = cwriter.lock().await;
            let to_write = dev.write(&buffer[..read]);
            if let Err(e) = to_write {
                error!("Failed to write packet due to {}", e);

                drop(dev);
                continue;
            }

            drop(dev);

            let wrote = to_write.unwrap();
            info!("Wrote {} bytes", wrote);
        }
    });

    let creader = reader.clone();
    let csocket = msocket.clone();
    tokio::spawn(async move {
        loop {
            trace!("Sending cycle");

            let mut buffer = [0; 4096];

            let mut dev = creader.lock().await;
            let to_read = dev.read(&mut buffer);
            if let Err(e) = to_read {
                error!("Failed to read packet due to {}", e);

                drop(dev);
                continue;
            }

            let read = to_read.unwrap();
            info!("Read {} bytes", read);
            drop(dev);

            if let Ok(_) = packet::ip::v4::Packet::new(&buffer[..read]) {
                debug!("Packet read from tun is IP");

                let socket = csocket.clone();
                let to_send = socket.send(&buffer[..read]).await;
                if let Err(e) = &to_send {
                    error!("Failed to send packet due to {}", e);
                }

                let sent = to_send.unwrap();

                info!("Sent {} bytes", sent);
            } else {
                debug!("Packet read from tun is not IP");
            }
        }
    });

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

    let socket = msocket.clone();
    loop {
        trace!("Waiting for ping interval");
        interval.tick().await;
        trace!("Ping time interval reached");

        let buffer = [0; 4096];
        if let Err(e) = socket.send(&buffer).await {
            error!("Failed to ping the server due {}", e);
        }

        info!("Ping");
    }
}
