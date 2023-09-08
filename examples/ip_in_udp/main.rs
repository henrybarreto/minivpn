use std::{
    fs::File,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
};

use packet::{Builder, Packet};

use serde::Deserialize;

extern crate tun;

#[derive(Debug, Deserialize)]
struct Data {
    pub name: String,
    pub host: [u8; 4],
    pub server: [u8; 4],
    pub peer: [u8; 4],
}

fn main() {
    let mut file = File::open("config.toml").expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");

    let d: Data = toml::from_str(&contents).unwrap();

    let mut config = tun::Configuration::default();
    config
        .address(Ipv4Addr::from(d.host))
        .netmask((255, 255, 255, 0))
        .up();

    let mut dev = tun::create(&config).unwrap();
    let mut buffer = [0; 4096];

    /*let mut socket = socket2::Socket::new_raw(
        socket2::Domain::IPV4,
        socket2::Type::RAW,
        Some(socket2::Protocol::UDP),
    )
    .unwrap();
    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::from(d.server), 0).into();

    socket.set_header_included(true).unwrap();
    socket.connect(&address).unwrap();*/

    let socket = UdpSocket::bind("0.0.0.0:8080").expect("couldn't bind to address");
    let address = format!(
        "{}.{}.{}.{}:{}",
        d.server[0], d.server[1], d.server[2], d.server[3], 8080
    );
    socket.connect(address).expect("connect function failed");

    loop {
        let read = dev.read(&mut buffer).unwrap();
        println!("read {} bytes", read);
        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            match ip.protocol() {
                _ => {
                    let mut port_src: u16;
                    let mut port_dst: u16;

                    match ip.protocol() {
                        packet::ip::Protocol::Udp => {
                            let udp = packet::udp::Packet::new(ip.payload()).unwrap();
                            println!("udp: {:?}", udp);

                            port_src = udp.source();
                            port_dst = udp.destination();
                        }
                        packet::ip::Protocol::Tcp => {
                            let tcp = packet::tcp::Packet::new(ip.payload()).unwrap();
                            println!("tcp: {:?}", tcp);

                            port_src = tcp.source();
                            port_dst = tcp.destination();
                        }
                        _ => {
                            port_src = 0;
                            port_dst = 0;
                        }
                    }

                    socket.send(&buffer[..read]).expect("couldn't send message");

                    let mut buf = [0; 4096];
                    let read = socket.recv(&mut buf).expect("didn't receive data");

                    // let resposne = packet::ip::v4::Builder::default()
                    //     .id(0x01)
                    //     .unwrap()
                    //     .source(Ipv4Addr::from(d.peer))
                    //     .unwrap()
                    //     .destination(Ipv4Addr::from(d.host))
                    //     .unwrap()
                    //     .udp()
                    //     .unwrap()
                    //     .source(8080)
                    //     .unwrap()
                    //     .destination(8080)
                    //     .unwrap()
                    //     .payload(&buf[..read])
                    //     .unwrap()
                    //     .build()
                    //     .unwrap();
                    let response = packet::ip::v4::Builder::default()
                        .id(0x01)
                        .unwrap()
                        .ttl(64)
                        .unwrap()
                        .source(ip.destination())
                        .unwrap()
                        .destination(ip.source())
                        .unwrap()
                        .udp()
                        .unwrap()
                        .source(port_dst)
                        .unwrap()
                        .destination(port_src)
                        .unwrap()
                        .payload(&buf[..read])
                        .unwrap()
                        .build()
                        .unwrap();

                    let wrote = dev.write(&response).unwrap();
                    println!("wrote to TUN0 {} bytes", wrote);

                    /*println!("ip: {:?}", ip);
                    println!("ip payload: {:?}", ip.payload());

                    let request = packet::ip::v4::Builder::default()
                        .id(0x01)
                        .unwrap()
                        .source(Ipv4Addr::new(201, 182, 197, 121))
                        .unwrap()
                        .destination(Ipv4Addr::new(104, 248, 33, 94))
                        .unwrap()
                        .udp()
                        .unwrap()
                        .source(8080)
                        .unwrap()
                        .destination(8080)
                        .unwrap()
                        .payload(&buffer[..read])
                        .unwrap()
                        .build()
                        .unwrap();

                    let wrote = socket.write(&request).unwrap();
                    println!("wrote {} bytes", wrote);

                    let read = socket.read(&mut buffer).unwrap();
                    println!("read {} bytes", read);

                    let pi = packet::ip::v4::Packet::new(&buffer[..read]).unwrap();
                    println!("ip: {:?}", pi);

                    let resposne = packet::ip::v4::Builder::default()
                        .id(0x01)
                        .unwrap()
                        .flags(pi.flags())
                        .unwrap()
                        .source(Ipv4Addr::new(104, 248, 33, 94))
                        .unwrap()
                        .destination(Ipv4Addr::new(201, 182, 197, 121))
                        .unwrap()
                        .udp()
                        .unwrap()
                        .source(8080)
                        .unwrap()
                        .destination(8080)
                        .unwrap()
                        .payload(&buffer[..read])
                        .unwrap()
                        .build()
                        .unwrap();

                    let wrote = dev.write(&resposne).unwrap();
                    println!("wrote {} bytes", wrote);*/
                }
            }
        }
    }
}
