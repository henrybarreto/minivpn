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

    //let socket = UdpSocket::bind("0.0.0.0:8080").expect("couldn't bind to address");
    let address = &args[2];
    let socket = TcpStream::connect(address).expect("couldn't bind to address");
    socket
        .set_nonblocking(true)
        .expect("set_nonblocking call failed");
    // socket.connect(address).expect("connect function failed");

    let mdev = Arc::new(Mutex::new(dev));
    let msocket = Arc::new(Mutex::new(socket));

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

            println!("IP {:?}", ip);
            let mut socket = csocket.lock().unwrap();
            dbg!(&socket);
            socket
                .write(&buffer[..read])
                .expect("couldn't send message");
            drop(socket);

            println!("WROTE TO SOCKET");
        } else {
            println!("PACKET NOT IP");
        }
    });

    let csocket = msocket.clone();
    let cdev = mdev.clone();
    thread::spawn(move || loop {
        let mut socket = csocket.lock().unwrap();

        let mut buffer = [0; 4096];
        let read = match socket.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                continue;
            }
        };
        drop(socket);

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
