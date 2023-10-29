use packet::Packet;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpListener, UdpSocket},
    string,
    sync::{Arc, Mutex},
    thread,
};

//if let None = origin {
//    println!("key {:?}", ip.source());
//    println!("value {:?}", addr.ip());

//    networks.insert(ip.source().into(), addr.ip());

//    println!("inserted key {:?} and {:?}", addr.ip(), ip.source());

//    if let None = peer {
//        csocket.send_to(&buffer, peer.unwrap().to_string()).unwrap();
//    }
//} else {
//    let mut value = peer.unwrap().clone().to_string();
//    println!(
//        "already inseted the key {:?} with value {:?}",
//        ip.source(),
//        value
//    );

//    value.push_str(":8080");

//    println!("with port {:?}", value);

//    csocket.send_to(&buffer, value).unwrap();
//}

fn main() {
    let mut networks = HashMap::<IpAddr, IpAddr>::new();
    let mnetworks = Arc::new(Mutex::new(networks));

    let socket = UdpSocket::bind("0.0.0.0:8081").unwrap();

    let msocket = Arc::new(socket);

    let cnetworks = mnetworks.clone();
    let csocket = msocket.clone();
    thread::spawn(move || loop {
        let mut buffer = [0; 4096];

        let (len, addr) = csocket.recv_from(&mut buffer).unwrap();
        println!("{:?} bytes received from {:?}", len, addr);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
            println!("PACKET IS IP");

            let mut n = cnetworks.lock().unwrap();

            let origin = n.get(&ip.source().into());

            if let None = origin {
                println!("key {:?}", ip.source());
                println!("value {:?}", addr.ip());

                n.insert(ip.source().into(), addr.ip());

                println!("inserted key {:?} and {:?}", addr.ip(), ip.source());
            }

            let peer = n.get(&ip.destination().into());

            if let None = peer {
                println!("peer is none");
            } else {
                let mut value = peer.unwrap().clone().to_string();
                println!(
                    "already inseted the key {:?} with value {:?}",
                    ip.source(),
                    value
                );

                value.push_str(":8080");

                println!("with port {:?}", value);

                let ip = packet::ip::v4::Packet::new(&buffer).unwrap();
                dbg!(ip);
                println!("sending to {:?}", value);
                println!("sending to {:?}", ip.destination());

                csocket.send_to(&buffer, value).unwrap();
            }
        } else {
            println!("PACKET NOT IP");
        }
    });

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
