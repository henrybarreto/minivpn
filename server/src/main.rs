use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use ipnet::Ipv4Net;
use packet::Builder;
use tokio::{
    net::UdpSocket,
    sync::{Mutex, RwLock},
};

#[tokio::main]
async fn main() {
    env_logger::init();

    let networks = HashMap::<IpAddr, SocketAddr>::new();
    let anetworks = Arc::new(RwLock::new(networks));

    let net = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap();

    let auth_networks = anetworks.clone();
    tokio::spawn(async move {
        dbg!("AUTH");
        // connect.
        let mut counter = 0;

        let socket = UdpSocket::bind("0.0.0.0:1120").await.unwrap();
        loop {
            let mut buffer = [0; 1];

            let (_, addr) = match socket.recv_from(&mut buffer).await {
                Ok((read, addr)) => (read, addr),
                Err(_) => continue,
            };

            let peer = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 100 + counter), 24).unwrap();
            socket
                .send_to(&bincode::serialize(&peer).unwrap(), addr)
                .await
                .unwrap();

            let mut networks = auth_networks.write().await;
            networks.insert(peer.addr().into(), addr);
            drop(networks);

            counter += 1;
        }
    });

    let networks = anetworks.clone();

    let socket = UdpSocket::bind("0.0.0.0:9807").await.unwrap();
    let msocket = Arc::new(socket);

    let csocket = msocket.clone();
    loop {
        dbg!("LOOP");
        let mut buffer = [0; 4096];

        let socket = csocket.clone();
        let (read, addr) = match socket.recv_from(&mut buffer).await {
            Ok((read, addr)) => (read, addr),
            Err(_) => continue,
        };

        dbg!("After read");

        let csocket = csocket.clone();
        let cmains = networks.clone();
        tokio::spawn(async move {
            dbg!("Spawn");
            let socket = csocket.clone();

            if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
                dbg!("IP");
                let source: IpAddr = ip.source().into();
                let destination: IpAddr = ip.destination().into();
                dbg!(source, destination);

                let mut networks = cmains.read().await;
                dbg!(&networks);

                let from = networks.get(&source);
                if let Some(a) = from {
                    if a != &addr {
                        dbg!("FROM NOT EQUAL");
                        dbg!(a, &addr);
                        drop(networks);

                        return;
                    }
                } else {
                    drop(networks);
                    dbg!("FROM NOT FOUND");

                    return;
                }

                if let Some(to) = networks.get(&destination) {
                    dbg!(&to);
                    match socket.send_to(&buffer[..read], to).await {
                        Ok(send) => {
                            if send == 0 {
                                dbg!("SEND 0");
                                //networks.remove(&destination);
                            }

                            dbg!("Send");
                        }
                        Err(e) => {
                            dbg!("ERROR", e);
                            // When the destination is not reachable, remove it from the list.
                            //networks.remove(&destination);
                        }
                    }

                    drop(networks);
                }
            } else {
                dbg!("PACKET NOT IP");
            }
        });
        dbg!("done");
    }
}
