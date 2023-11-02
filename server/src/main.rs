use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex, RwLock},
};

// async fn handle(mut socket: TcpStream, cnetworks: Arc<Mutex<HashMap<IpAddr, TcpStream>>>) {
//     tokio::spawn(async move {
//         loop {
//             let mut buffer = [0; 4096];
//
//             let _ = match socket.read(&mut buffer).await {
//                 Ok(read) => {
//                     if read == 0 {
//                         break;
//                     }
//
//                     read
//                 }
//                 Err(_) => break,
//             };
//
//             //     if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
//             //         let source: IpAddr = ip.source().into();
//             //         let destination: IpAddr = ip.destination().into();
//
//             //         let mut networks = cnetworks.lock().unwrap();
//             //         dbg!(&networks);
//
//             //         networks.insert(source, socket.try_clone().unwrap());
//
//             //         if let Some(to) = networks.get(&destination) {
//             //             match to.try_clone() {
//             //                 Ok(mut dest) => {
//             //                     dest.write(&buffer).unwrap();
//             //                 }
//             //                 Err(_) => {
//             //                     networks.remove(&destination);
//             //                 }
//             //             }
//
//             //             drop(networks);
//             //         }
//             //     } else {
//             //         dbg!("PACKET NOT IP");
//             //     }
//         }
//     });
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let networks = HashMap::<IpAddr, Arc<RwLock<TcpStream>>>::new();
    let mnetworks = Arc::new(RwLock::new(networks));

    let listener = TcpListener::bind("0.0.0.0:8081").await?;
    let (tx, mut rx) = mpsc::channel::<IpAddr>(100);

    let anetworks = mnetworks.clone();
    tokio::spawn(async move {
        loop {
            let bnetworks = anetworks.clone();

            let addr = rx.recv().await.unwrap();

            tokio::spawn(async move {
                loop {
                    let mut buffer = [0; 4096];

                    let networks = bnetworks.read().await;
                    {
                        let mutex = match networks.get(&addr) {
                            Some(mutex) => mutex,
                            None => break,
                        };

                        let mut socket = mutex.write().await;

                        let _ = match socket.read(&mut buffer).await {
                            Ok(read) => {
                                if read == 0 {
                                    break;
                                }

                                read
                            }
                            Err(_) => break,
                        };
                    }

                    if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
                        dbg!(ip);
                        // let source: IpAddr = ip.source().into();
                        let destination: IpAddr = ip.destination().into();

                        if let Some(to) = networks.get(&destination) {
                            /*match to {
                                Ok(mut dest) => {
                                    dest.write(&buffer).unwrap();
                                }
                                Err(_) => {
                                    networks.remove(&destination);
                                }
                            }*/
                            // let mutex = to.clone();
                            // let dest = mutex.lock();

                            // dest.await.write(&buffer).await.unwrap();

                            // drop(networks);
                        } else {
                            dbg!("NO DESTINATION");
                        }
                    }
                }
            });
        }
    });

    let anetworks = mnetworks.clone();
    loop {
        let cnetworks = anetworks.clone();

        match listener.accept().await {
            Ok((mut socket, _)) => {
                let mut buffer = [0; 4096];

                match socket.read(&mut buffer).await {
                    Ok(read) => {
                        if read == 0 {
                            continue;
                        }

                        read
                    }
                    Err(_) => continue,
                };

                let addr = match bincode::deserialize::<IpAddr>(&buffer) {
                    Ok(addr) => addr,
                    Err(_) => continue,
                };

                cnetworks
                    .write()
                    .await
                    .insert(addr, Arc::new(RwLock::new(socket)));
                drop(cnetworks);

                tx.send(addr).await.unwrap();
            }
            Err(e) => {
                dbg!(e);

                continue;
            }
        }
    }
}

// tokio::spawn(async move {
//     loop {
//         // let mut networks = cnetworks.read().unwrap();
//         // dbg!(&networks);

//         let mut buffer = [0; 4096];

//         let socket = cnetworks
//             .read()
//             .unwrap()
//             .get(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 100)))
//             .unwrap()
//             .clone();

//         /*let mut socket = networks
//             .get(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 100)))
//             .unwrap()
//             .clone();
//         .get(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 100)))*/
//         let a = socket.read().unwrap();
//         //a.read(&mut buffer).await.unwrap();

//         // let _ = match a.read(&mut buffer).await {
//         //     Ok(read) => {
//         //         if read == 0 {
//         //             break;
//         //         }

//         //         read
//         //     }
//         //     Err(_) => break,
//         // };

//         // if let Ok(ip) = packet::ip::v4::Packet::new(&buffer) {
//         //     let source: IpAddr = ip.source().into();
//         //     let destination: IpAddr = ip.destination().into();

//         //     let mut networks = cnetworks.lock().unwrap();
//         //     dbg!(&networks);

//         //     // networks.insert(source, &socket);

//         //     if let Some(to) = networks.get(&destination) {
//         //         /*match to {
//         //             Ok(mut dest) => {
//         //                 dest.write(&buffer).unwrap();
//         //             }
//         //             Err(_) => {
//         //                 networks.remove(&destination);
//         //             }
//         //         }*/
//         //         to.write(&buffer);

//         //         drop(networks);
//         //     }
//         // } else {
//         //     dbg!("PACKET NOT IP");
//         // }
//     }
// });
//
