use std::{
    env,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpStream, UdpSocket},
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
};

use bincode;
use packet::{Builder, Packet};
use serde::{Deserialize, Serialize};

extern crate tun;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Identity {
    pub network: u64,
}

fn main() {
    let mut config = tun::Configuration::default();
    let args: Vec<String> = env::args().collect();

    println!("args: {:?}", args);
    config
        // .address(Ipv4Addr::new(10, 0, 0, 100))
        .address(Ipv4Addr::from_str(&args[1]).unwrap())
        .netmask((255, 255, 255, 0))
        .up();

    let dev = tun::create(&config).unwrap();
    dev.set_nonblock().unwrap();

    let socket = UdpSocket::bind("0.0.0.0:8080").expect("couldn't bind to address");
    let address = &args[2];
    socket.connect(address).expect("connect function failed");

    let mdev = Arc::new(Mutex::new(dev));
    let msocket = Arc::new(socket);

    let mut csocket = msocket.clone();
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

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            println!("PACKET IS IP");

            let socket = &mut csocket;
            socket.send(&buffer[..read]).expect("couldn't send message");
            println!("WROTE TO SOCKET");
        } else {
            println!("PACKET NOT IP");
        }
    });

    let csocket = msocket.clone();
    let cdev = mdev.clone();
    thread::spawn(move || loop {
        let socket = &csocket;

        let mut buffer = [0; 4096];
        dbg!("B");
        let read = match socket.recv(&mut buffer) {
            Ok(read) => {
                dbg!("D");

                read
            }
            Err(_) => {
                continue;
            }
        };
        dbg!("C");

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            println!("PACKET IS IP");

            let mut dev = cdev.lock().unwrap();
            match dev.write(&buffer[..read]) {
                Ok(usize) => {
                    println!("WROTE TO DEV");
                    println!("WROTE {:?} BYTES", usize);
                    dbg!(ip);
                }
                Err(e) => {
                    println!("ERROR WRITING TO DEV {:?}", e);
                    continue;
                }
            }
            drop(dev);
        } else {
            println!("PACKET NOT IP");
        }
    });

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

// let mut dev = cdev.lock().unwrap();
// println!("WRITING TO DEV");
// match dev.write_all(&response) {
//     Ok(_) => {}
//     Err(_) => {
//         continue;
//     }
// }
// println!("WROTE TO DEV");
//drop(dev);
// match ip.protocol() {
//     packet::ip::Protocol::Udp => {
//         let udp = packet::udp::Packet::new(ip.payload()).unwrap();

//         println!("CREATING PACKET");
//         let response = packet::ip::v4::Builder::default()
//             .id(0x01)
//             .unwrap()
//             .ttl(64)
//             .unwrap()
//             .source(ip.source())
//             .unwrap()
//             .destination(ip.destination())
//             .unwrap()
//             .udp()
//             .unwrap()
//             .source(udp.source())
//             .unwrap()
//             .destination(udp.destination())
//             .unwrap()
//             .payload(udp.payload())
//             .unwrap()
//             .build()
//             .unwrap();

//         let mut dev = cdev.lock().unwrap();
//         println!("WRITING TO DEV");
//         match dev.write_all(&response) {
//             Ok(_) => {}
//             Err(_) => {
//                 continue;
//             }
//         }
//         println!("WROTE TO DEV");
//         drop(dev);
//     }
//     packet::ip::Protocol::Tcp => {
//         println!("PACKET TYPE TCP");
//     }
//     _ => {
//         println!("PACKET TYPE NOT SUPPORTED");
//     }
// }
