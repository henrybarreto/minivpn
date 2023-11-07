use std::env;
use std::io::{Read, Write};
use std::sync::Arc;

use clap::{Arg, Command};
use futures::StreamExt;
use ipnet::Ipv4Net;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{net::UdpSocket, sync::Mutex};
use tun::Device;

extern crate tun;

#[tokio::main]
async fn main() {
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
    let mut config = tun::Configuration::default();

    let socket = UdpSocket::bind("0.0.0.0:1620")
        .await
        .expect("couldn't bind to address");

    socket
        .send_to(&[0; 1], format!("{}:{}", server, "1120"))
        .await
        .unwrap();

    let mut buf = [0; 4096];
    let (read, _) = socket.recv_from(&mut buf).await.unwrap();

    let me: Ipv4Net = bincode::deserialize(&buf[..read]).unwrap();
    dbg!(&me);

    config.address(me.addr()).netmask(me.netmask()).up();
    config.queues(2);

    let dev = tun::create(&config).unwrap();
    let (reader, writer) = dev.split();

    let router = format!("{}:{}", server, port);
    dbg!(&router);

    socket
        .connect(router)
        .await
        .expect("couldn't connect to address");

    let reader = Arc::new(Mutex::new(reader));
    let writer = Arc::new(Mutex::new(writer));

    let msocket = Arc::new(socket);

    let cwriter = writer.clone();
    let csocket = msocket.clone();
    tokio::spawn(async move {
        println!("LOOP");
        loop {
            println!("IN LOOP");
            let mut buffer = [0; 4096];

            let socket = csocket.clone();
            println!("WAITING FOR SOCKET BEFORE LOCK RECV");
            let read = match socket.recv(&mut buffer).await {
                Ok(read) => {
                    if read == 0 {
                        drop(socket);

                        break;
                    }

                    read
                }
                Err(e) => {
                    dbg!(e);
                    drop(socket);

                    continue;
                }
            };
            dbg!("RECEIVED PACKET");
            dbg!(read);
            drop(socket);

            println!("WAITING FOR LOCK ON DEV TO WRITE");
            let mut dev = cwriter.lock().await;
            println!("GOT LOCK ON DEV TO WRITE");
            dbg!("WRITE");

            let wrote = match dev.write(&buffer) {
                Ok(wrote) => wrote,
                Err(e) => {
                    drop(dev);
                    continue;
                }
            };
            dbg!(wrote);
            drop(dev);
        }
    });

    let creader = reader.clone();
    let csocket = msocket.clone();
    tokio::spawn(async move {
        loop {
            let mut buffer = [0; 4096];

            let mut dev = creader.lock().await;

            let read = match dev.read(&mut buffer) {
                Ok(read) => read,
                Err(e) => {
                    drop(dev);
                    continue;
                }
            };
            dbg!(read);
            drop(dev);

            if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
                dbg!(ip);

                let socket = csocket.clone();
                if let Err(e) = socket.send(&buffer).await {
                    dbg!(e);
                }

                dbg!("SENT PACKET FROM SOCKET");
            } else {
                dbg!("PACKET NOT IP");
            }
        }
    });

    let csocket = msocket.clone();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

    loop {
        interval.tick().await;

        let socket = csocket.clone();
        let buffer = [0; 1];
        socket.send(&buffer).await.unwrap();
    }
}
