use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpListener, TcpStream, UdpSocket},
    string,
    sync::{Arc, Mutex},
    thread,
};

fn main() {
    let mut networks = HashMap::<IpAddr, TcpStream>::new();
    let mnetworks = Arc::new(Mutex::new(networks));

    let listener = TcpListener::bind("0.0.0.0:8081").unwrap();

    for socket in listener.incoming() {
        let msocket = Arc::new(Mutex::new(socket.unwrap()));

        let cnetworks = mnetworks.clone();
        let csocket = msocket.clone();
        thread::spawn(move || loop {
            let mut buffer = [0; 4096];

            let mut l = csocket.lock().unwrap();
            let read = l.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            println!(
                "{:?} bytes received from {:?}",
                read,
                l.peer_addr().unwrap()
            );

            if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
                println!("PACKET IS IP");

                let mut n = cnetworks.lock().unwrap();
                dbg!(&n);

                let origin = n.get(&ip.source().into());
                dbg!(&origin);

                if let None = origin {
                    println!("key {:?}", ip.source());
                    println!("value {:?}", l.peer_addr().unwrap().ip());

                    n.insert(ip.source().into(), l.try_clone().unwrap());

                    println!(
                        "inserted key {:?} and {:?}",
                        l.peer_addr().unwrap().ip(),
                        ip.source()
                    );
                }

                let peer = n.get(&ip.destination().into());

                if let None = peer {
                    println!("peer is none");
                } else {
                    println!("peer is some");
                    let mut dest = peer.unwrap().try_clone().unwrap();

                    println!(
                        "already inseted the key {:?} with value {:?}",
                        ip.source(),
                        dest.peer_addr().unwrap().ip()
                    );

                    // value.push_str(":8080");

                    println!("destination {:?}", dest.peer_addr().unwrap().ip());

                    let ip = packet::ip::v4::Packet::new(&buffer).unwrap();
                    dbg!(ip);

                    dest.write(&buffer).unwrap();
                    //csocket.write(&buffer, value).unwrap();
                }
            } else {
                println!("PACKET NOT IP");
            }
        });
    }

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
