use std::io::{Read, Write};

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
        println!("read {} bytes", read);

        let p = packet::ip::Packet::new(&buf[..read]).unwrap();

        if let Ok(udp) = packet::udp::Packet::new(p.payload()) {
            println!("udp packet: {:?}", udp);

            let ip = packet::ip::v4::Packet::new(&buf[..read]).unwrap();

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
                .payload(udp.payload())
                .unwrap()
                .build()
                .unwrap();

            dev.write(&response).unwrap();
        }

        /*let p = packet::ip::Packet::new(&buf[..read]).unwrap();

        if let Ok(icmp) = packet::icmp::Packet::new(p.payload()) {
            println!("icmp packet: {:?}", icmp);

            let echo = icmp.echo().unwrap();

            let ip = packet::ip::v4::Packet::new(&buf[..read]).unwrap();

            let reply = packet::ip::v4::Builder::default()
                .id(0x42)
                .unwrap()
                .ttl(64)
                .unwrap()
                .source(ip.destination())
                .unwrap()
                .destination(ip.source())
                .unwrap()
                .icmp()
                .unwrap()
                .echo()
                .unwrap()
                .reply()
                .unwrap()
                .identifier(echo.identifier())
                .unwrap()
                .sequence(echo.sequence())
                .unwrap()
                .payload(echo.payload())
                .unwrap()
                .build()
                .unwrap();

            let wrote = dev.write(&reply).unwrap();
            println!("wrote {} bytes", wrote);
        }*/
    }
}
