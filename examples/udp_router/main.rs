use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4},
};

use packet::{self, Builder, Packet};

fn main() {
    let mut config = tun::Configuration::default();

    config
        .name("minivpn0")
        .address((10, 0, 0, 1))
        .netmask((255, 255, 255, 0))
        .up();

    let mut dev = tun::create(&config).unwrap();
    let mut buf = [0; 4096];

    loop {
        let read = dev.read(&mut buf).unwrap();

        if let Ok(ip) = packet::ip::v4::Packet::new(&buf[..read]) {
            if let Ok(udp) = packet::udp::Packet::new(ip.payload()) {
                println!("udp packet: {:?}", udp);

                let mut socket =
                    socket2::Socket::new_raw(socket2::Domain::IPV4, socket2::Type::DGRAM, None)
                        .unwrap();

                println!("udp source port: {}", udp.source());
                println!("udp destination port: {}", udp.destination());

                let address: socket2::SockAddr =
                    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), udp.destination()).into();

                println!("connecting to {:?}", address);
                socket.connect(&address.into()).unwrap();
                println!("connected");

                println!("wroting {} bytes", udp.payload().len());
                let wrote = socket.write(&udp.payload()).unwrap();
                println!("wrote {} bytes", wrote);

                let mut buffer = [0; 4096];
                let read = socket.read(&mut buffer).unwrap();
                println!("read {} bytes", read);
                println!("buffer: {:?}", &buffer[..read]);
                println!("string: {}", std::str::from_utf8(&buffer[..read]).unwrap());

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
                    .source(udp.destination())
                    .unwrap()
                    .destination(udp.source())
                    .unwrap()
                    .payload(&buffer[..read])
                    .unwrap()
                    .build()
                    .unwrap();

                dev.write(&response).unwrap();
            }
        }
    }
}
