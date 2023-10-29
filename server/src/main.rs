use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{IpAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

fn main() {
    let networks = HashMap::<IpAddr, TcpStream>::new();
    let mnetworks = Arc::new(Mutex::new(networks));

    let listener = TcpListener::bind("0.0.0.0:8081").unwrap();

    for socket in listener.incoming() {
        let msocket = Arc::new(Mutex::new(socket.unwrap()));

        let cnetworks = mnetworks.clone();
        let csocket = msocket.clone();
        thread::spawn(move || loop {
            let mut buffer = [0; 4096];

            let mut socket = csocket.lock().unwrap();
            let _ = match socket.read(&mut buffer) {
                Ok(read) => {
                    if read == 0 {
                        break;
                    }

                    read
                }
                Err(_) => break,
            };

            if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
                let source: IpAddr = ip.source().into();
                let destination: IpAddr = ip.destination().into();

                let mut networks = cnetworks.lock().unwrap();
                dbg!(&networks);

                if let None = networks.get(&ip.source().into()) {
                    networks.insert(source, socket.try_clone().unwrap());
                }

                if let Some(to) = networks.get(&destination) {
                    let mut dest = to.try_clone().unwrap();

                    dest.write(&buffer).unwrap();
                }
            } else {
                dbg!("PACKET NOT IP");
            }
        });
    }

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
