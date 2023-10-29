use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{IpAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

fn handle(mut socket: TcpStream, cnetworks: Arc<Mutex<HashMap<IpAddr, TcpStream>>>) {
    thread::spawn(move || loop {
        let mut buffer = [0; 4096];

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

            networks.insert(source, socket.try_clone().unwrap());

            if let Some(to) = networks.get(&destination) {
                match to.try_clone() {
                    Ok(mut dest) => {
                        dest.write(&buffer).unwrap();
                    }
                    Err(_) => {
                        networks.remove(&destination);
                    }
                }

                drop(networks);
            }
        } else {
            dbg!("PACKET NOT IP");
        }
    });
}

fn main() {
    let networks = HashMap::<IpAddr, TcpStream>::new();
    let mnetworks = Arc::new(Mutex::new(networks));

    let listener = TcpListener::bind("0.0.0.0:8081").unwrap();

    for stream in listener.incoming() {
        let cnetworks = mnetworks.clone();
        match stream {
            Ok(stream) => handle(stream.try_clone().unwrap(), cnetworks),
            Err(e) => {
                dbg!(e);

                continue;
            }
        }
    }
}
