use std::collections::HashMap;
use std::sync::mpsc::{self, channel, Receiver, Sender};
use std::{
    fs::File,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{Arc, Mutex},
    thread,
};

use packet::{Builder, Packet};

use serde::Deserialize;

extern crate tun;

#[derive(Debug)]
struct Peer {
    pub host: Ipv4Addr,   // IP of the peer.
    pub server: Ipv4Addr, // IP of the server.
}

#[derive(Debug)]
pub struct Connection {
    pub address: Ipv4Addr,
    pub src: u16,
    pub dst: u16,
}

#[derive(Debug, Deserialize)]
struct Data {
    pub name: String,
    pub host: [u8; 4],
    pub server: [u8; 4],
    pub peer: [u8; 4],
}

fn main() {
    let mut config = tun::Configuration::default();
    config
        .address(Ipv4Addr::new(10, 0, 0, 1))
        .netmask((255, 255, 255, 0))
        .up();

    let dev = tun::create(&config).unwrap();
    dev.set_nonblock().unwrap();

    let socket = UdpSocket::bind("0.0.0.0:8080").expect("couldn't bind to address");
    let address = "172.17.0.2:8080";
    socket.connect(address).expect("connect function failed");

    let mdev = Arc::new(Mutex::new(dev));
    let msocket = Arc::new(socket);

    let connections = Arc::new(Mutex::new(HashMap::<(Ipv4Addr, u16), Connection>::new()));

    let cconnections = connections.clone();
    let csocket = msocket.clone();
    let cdev = mdev.clone();
    thread::spawn(move || loop {
        let mut buffer = [0; 4096];

        let mut dev = cdev.lock().unwrap();
        let read: usize = match dev.read(&mut buffer) {
            Ok(read) => read,
            Err(_) => {
                continue;
            }
        };
        drop(dev);

        println!("read from dev {:?} bytes", &buffer[..read]);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            match ip.protocol() {
                packet::ip::Protocol::Udp => {
                    let udp = packet::udp::Packet::new(ip.payload()).unwrap();

                    let mut c = cconnections.lock().unwrap();
                    c.insert(
                        (ip.destination(), udp.destination()),
                        Connection {
                            address: ip.destination(),
                            src: udp.source(),
                            dst: udp.destination(),
                        },
                    );
                    drop(c);

                    let socket = &csocket;
                    socket.send(&buffer[..read]).expect("couldn't send message");
                    println!("WROTE TO SOCKET");
                }
                _ => {
                    println!("PACKET TYPE NOT SUPPORTED");
                }
            }
        } else {
            println!("PACKET NOT IP");
        }
    });

    let cconnections = connections.clone();
    let csocket = msocket.clone();
    let cdev = mdev.clone();
    thread::spawn(move || loop {
        let socket = &csocket;

        let mut buffer = [0; 4096];
        let read = match socket.recv(&mut buffer) {
            Ok(read) => read,
            Err(_) => {
                continue;
            }
        };
        println!("read from socket {:?} bytes", &buffer[..read]);

        let mut c = cconnections.lock().unwrap();
        let connection = c.get(&(Ipv4Addr::new(10, 0, 0, 100), 8080)).unwrap();
        let port_src = connection.src;
        let port_dst = connection.dst;
        let address = connection.address;
        drop(c);

        println!("CREATING PACKET");
        let response = packet::ip::v4::Builder::default()
            .id(0x01)
            .unwrap()
            .ttl(64)
            .unwrap()
            .source(address)
            .unwrap()
            .destination(Ipv4Addr::new(10, 0, 0, 1))
            .unwrap()
            .udp()
            .unwrap()
            .source(port_dst)
            .unwrap()
            .destination(port_src)
            .unwrap()
            .payload(&buffer)
            .unwrap()
            .build()
            .unwrap();

        let mut dev = cdev.lock().unwrap();
        println!("WRITING TO DEV");
        match dev.write_all(&response) {
            Ok(_) => {}
            Err(_) => {
                continue;
            }
        }
        println!("WROTE TO DEV");
        drop(dev);
    });

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
