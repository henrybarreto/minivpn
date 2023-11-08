use clap::{Arg, Command};
use ipnet::Ipv4Net;
use log::{debug, error, info, trace};
use std::env;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::{net::UdpSocket, sync::Mutex};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_env("LOG")
        .init();

    let matches = Command::new("Orbit")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::new("address")
                .long("address")
                .short('a')
                .help("Server address")
                .value_name("ADDRESS")
                .required(true),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .help("Server port")
                .value_name("PORT")
                .default_value("9807"),
        )
        .get_matches();

    let server = matches.get_one::<String>("address").unwrap();
    let port = matches.get_one::<String>("port").unwrap();

    connect(server, port).await;
}

async fn connect(server: &str, port: &str) {
    info!("Obirt connecting to {}:{}", server, port);

    let mut config = tun::Configuration::default();

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("couldn't bind to address");

    info!("Bound to 1620");

    info!("Registering perr on the server");
    socket
        .send_to(&[0; 1], format!("{}:{}", server, "1120"))
        .await
        .unwrap();

    debug!("Waiting for server to respond");

    let mut buf = [0; 4096];
    let (read, _) = socket.recv_from(&mut buf).await.unwrap();

    info!("Received response from server {}", read);

    let me: Ipv4Net = bincode::deserialize(&buf[..read]).unwrap();

    info!("Peer registered as {}", me);

    config.address(me.addr()).netmask(me.netmask()).up();
    config.queues(2);

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
        interval.tick().await;
        trace!("Pinging server");

        let buffer = [0; 4096];
        if let Err(e) = socket.send(&buffer).await {
            panic!("Failed to ping the server due {}", e);
        }

        info!("Ping");
    }
}
