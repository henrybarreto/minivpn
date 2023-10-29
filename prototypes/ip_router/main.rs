use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4},
};

use etherparse;
use log::info;

use packet::{self, Builder, Packet};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut config = tun::Configuration::default();

    config
        .name("minivpn0")
        .address((10, 0, 0, 1))
        .netmask((255, 255, 255, 0))
        .up();

    let mut dev = tun::create(&config).unwrap();
    let mut buf = [0; 4096];

    let mut socket = socket2::Socket::new_raw(
        socket2::Domain::IPV4,
        socket2::Type::RAW,
        Some(socket2::Protocol::TCP),
    )
    .unwrap();

    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 0), 103).into();

    socket.set_header_included(true).unwrap();

    socket.connect(&address).unwrap();

    loop {
        let read = dev.read(&mut buf).unwrap();

        if let Ok(ip) = packet::ip::v4::Packet::new(&buf[..read]) {
            match ip.protocol() {
                packet::ip::Protocol::Tcp => {
                    let tcp = packet::tcp::Packet::new(ip.payload()).unwrap();

                    let request = packet::ip::v4::Builder::default()
                        .id(0x01)
                        .unwrap()
                        .ttl(64)
                        .unwrap()
                        .offset(ip.offset())
                        .unwrap()
                        .source(Ipv4Addr::new(192, 168, 1, 103))
                        .unwrap()
                        .destination(Ipv4Addr::new(192, 168, 1, 101))
                        .unwrap()
                        .protocol(packet::ip::Protocol::Tcp)
                        .unwrap()
                        .tcp()
                        .unwrap()
                        .source(tcp.source())
                        .unwrap()
                        .destination(8080)
                        .unwrap()
                        .sequence(tcp.sequence())
                        .unwrap()
                        .acknowledgment(tcp.acknowledgment())
                        .unwrap()
                        .window(tcp.window())
                        .unwrap()
                        .flags(tcp.flags())
                        .unwrap()
                        .payload(tcp.payload())
                        .unwrap()
                        .build()
                        .unwrap();

                    let p = packet::ip::v4::Packet::new(&request[..request.len()]).unwrap();
                    let t = packet::tcp::Packet::new(p.payload()).unwrap();

                    info!("---------------------------------------");

                    println!("t {:?}", t);

                    info!("writing {:?} bytes", request);
                    let wrote = socket.write(&request[..request.len()]).unwrap();
                    info!("wrote {} bytes", wrote);

                    if t.flags() == packet::tcp::Flags::ACK {
                        let i = packet::ip::v4::Builder::default()
                            .id(0x01)
                            .unwrap()
                            .ttl(64)
                            .unwrap()
                            .offset(p.offset())
                            .unwrap()
                            .source(Ipv4Addr::new(192, 168, 1, 103))
                            .unwrap()
                            .destination(Ipv4Addr::new(192, 168, 1, 101))
                            .unwrap()
                            .protocol(packet::ip::Protocol::Tcp)
                            .unwrap()
                            .tcp()
                            .unwrap()
                            .source(t.source())
                            .unwrap()
                            .destination(t.destination())
                            .unwrap()
                            .sequence(t.sequence())
                            .unwrap()
                            .acknowledgment(t.acknowledgment())
                            .unwrap()
                            .window(t.window())
                            .unwrap()
                            .flags(packet::tcp::Flags::ACK | packet::tcp::Flags::PSH)
                            .unwrap()
                            .payload(b"Hello World!")
                            .unwrap()
                            .build()
                            .unwrap();

                        let p = packet::ip::v4::Packet::new(&i[..i.len()]).unwrap();
                        let t = packet::tcp::Packet::new(p.payload()).unwrap();

                        socket.write(&i[..i.len()]).unwrap();
                        info!("wrote {} bytes", wrote);
                        info!("-----------------22222222222222222222222222222");
                    }

                    let mut buffer = [0; 4096];
                    let read = socket.read(&mut buffer).unwrap();
                    info!("read {} bytes", read);

                    info!("buffer {:?}", &buffer[..read]);

                    let pi = packet::ip::v4::Packet::new(&buffer[..read]).unwrap();
                    let pct = packet::tcp::Packet::new(pi.payload()).unwrap();

                    info!("pct {:?}", pct);
                    info!("pi {:?}", pi);

                    let response = packet::ip::v4::Builder::default()
                        .id(pi.id())
                        .unwrap()
                        .ttl(pi.ttl())
                        .unwrap()
                        .source(ip.destination()) // GATEWAY
                        .unwrap()
                        .destination(ip.source())
                        .unwrap()
                        .flags(pi.flags())
                        .unwrap()
                        .offset(pi.offset())
                        .unwrap()
                        .protocol(packet::ip::Protocol::Tcp)
                        .unwrap()
                        .tcp()
                        .unwrap()
                        .source(tcp.destination())
                        .unwrap()
                        .destination(tcp.source())
                        .unwrap()
                        .sequence(pct.sequence())
                        .unwrap()
                        .acknowledgment(pct.acknowledgment())
                        .unwrap()
                        .window(pct.window())
                        .unwrap()
                        .flags(pct.flags())
                        .unwrap()
                        .payload(pct.payload())
                        .unwrap()
                        .build()
                        .unwrap();

                    let p = packet::ip::v4::Packet::new(&response[..response.len()]).unwrap();
                    let t = packet::tcp::Packet::new(p.payload()).unwrap();

                    info!("p {:?}", p);
                    info!("t {:?}", t);

                    info!("---------------------------------------");

                    dev.write(&response[..response.len()]).unwrap();
                }
                /*packet::ip::Protocol::Icmp => {
                    let mut socket = socket2::Socket::new_raw(
                        socket2::Domain::IPV4,
                        socket2::Type::RAW,
                        Some(socket2::Protocol::ICMPV4),
                    )
                    .unwrap();

                    // 172.17.0.2
                    let address: socket2::SockAddr =
                        SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 100), 0).into();

                    socket.set_header_included(true).unwrap();

                    socket.connect(&address).unwrap();

                    let icmp = packet::icmp::Packet::new(ip.payload()).unwrap();

                    let request = packet::ip::v4::Builder::default()
                        .id(0x01)
                        .unwrap()
                        .ttl(64)
                        .unwrap()
                        .source(Ipv4Addr::new(0, 0, 0, 0))
                        .unwrap()
                        .destination(Ipv4Addr::new(192, 168, 1, 100))
                        .unwrap()
                        .icmp()
                        .unwrap()
                        .echo()
                        .unwrap()
                        .request()
                        .unwrap()
                        .identifier(icmp.echo().unwrap().identifier())
                        .unwrap()
                        .sequence(icmp.echo().unwrap().sequence())
                        .unwrap()
                        // .payload(icmp.echo().unwrap().payload())
                        // .unwrap()
                        .build()
                        .unwrap();

                    info!("writing {:?} bytes", request);
                    let wrote = socket.write(&request[..request.len()]).unwrap();
                    info!("wrote {} bytes", wrote);

                    let mut buffer = [0; 4096];
                    let read = socket.read(&mut buffer).unwrap();
                    info!("read {} bytes", read);

                    let pi = packet::ip::v4::Packet::new(&buffer[..read]).unwrap();
                    let pmci = packet::icmp::Packet::new(pi.payload()).unwrap();

                    let response = packet::ip::v4::Builder::default()
                        .id(0x01)
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
                        .identifier(pmci.echo().unwrap().identifier())
                        .unwrap()
                        .sequence(pmci.echo().unwrap().sequence())
                        .unwrap()
                        // .payload(pmci.echo().unwrap().payload())
                        // .unwrap()
                        .build()
                        .unwrap();

                    info!("response {:?} bytes", response);

                    dev.write(&response[..response.len()]).unwrap();
                }
                packet::ip::Protocol::Udp => {
                    if let Ok(udp) = packet::udp::Packet::new(ip.payload()) {
                        let mut socket = socket2::Socket::new_raw(
                            socket2::Domain::IPV4,
                            socket2::Type::RAW,
                            Some(socket2::Protocol::UDP),
                        )
                        .unwrap();

                        let address: socket2::SockAddr =
                            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0).into();

                        info!("ip: {:?}", ip);
                        info!("udp: {:?}", udp);

                        info!("udp destination: {}", udp.destination());

                        let request = packet::ip::v4::Builder::default()
                            .id(0x01)
                            .unwrap()
                            .ttl(64)
                            .unwrap()
                            .source(Ipv4Addr::new(0, 0, 0, 0))
                            .unwrap()
                            .destination(Ipv4Addr::new(127, 0, 0, 1))
                            .unwrap()
                            .udp()
                            .unwrap()
                            .source(udp.source())
                            .unwrap()
                            .destination(udp.destination())
                            .unwrap()
                            .payload(udp.payload())
                            .unwrap()
                            .build()
                            .unwrap();

                        socket.set_header_included(true).unwrap();

                        socket.connect(&address).unwrap();

                        info!("writing {:?} bytes", request);
                        let wrote = socket.write(&request[..request.len()]).unwrap();
                        info!("wrote {} bytes", wrote);

                        let mut buffer = [0; 4096];
                        let read = socket.read(&mut buffer).unwrap();
                        info!("read {} bytes", read);
                        info!("buffer: {:?}", &buffer[..read]);

                        let pi = packet::ip::v4::Packet::new(&buffer[..read]).unwrap();
                        let pdu = packet::udp::Packet::new(pi.payload()).unwrap();

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
                            .payload(pdu.payload())
                            .unwrap()
                            .build()
                            .unwrap();

                        dev.write(&response[..response.len()]).unwrap();
                    }
                }*/
                /*packet::ip::Protocol::Tcp => {
                    if let Ok(tcp) = packet::tcp::Packet::new(ip.payload()) {
                        let mut socket = socket2::Socket::new_raw(
                            socket2::Domain::IPV4,
                            socket2::Type::RAW,
                            Some(socket2::Protocol::TCP),
                        )
                        .unwrap();

                        let address: socket2::SockAddr =
                            SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0).into();

                        let request = packet::ip::v4::Builder::default()
                            .id(0x01)
                            .unwrap()
                            .ttl(64)
                            .unwrap()
                            .source(Ipv4Addr::new(0, 0, 0, 0))
                            .unwrap()
                            .destination(Ipv4Addr::new(127, 0, 0, 1))
                            .unwrap()
                            .tcp()
                            .unwrap()
                            .source(tcp.source())
                            .unwrap()
                            .destination(tcp.destination())
                            .unwrap()
                            .payload(tcp.payload())
                            .unwrap()
                            .flags(tcp.flags())
                            .unwrap()
                            .build()
                            .unwrap();

                        info!(">>>> tcp: {:?}", tcp);

                        socket.set_header_included(true).unwrap();

                        socket.connect(&address).unwrap();

                        info!("writing {:?} bytes", request);
                        let wrote = socket.write(&request[..request.len()]).unwrap();
                        info!("wrote {} bytes", wrote);

                        let mut buffer = [0; 4096];
                        let read = socket.read(&mut buffer).unwrap();
                        info!("read {:?} bytes", read);

                        let pi = packet::ip::v4::Packet::new(&buffer[..read]).unwrap();
                        info!("<<<< pi: {:?}", pi);
                        let pct = packet::tcp::Packet::new(pi.payload()).unwrap();
                        info!("<<<< pct: {:?}", pct);

                        info!("----------------------------------");

                        let response = packet::ip::v4::Builder::default()
                            .id(0x01)
                            .unwrap()
                            .ttl(64)
                            .unwrap()
                            .flags(pi.flags())
                            .unwrap()
                            .source(ip.destination())
                            .unwrap()
                            .destination(ip.source())
                            .unwrap()
                            .tcp()
                            .unwrap()
                            .source(tcp.destination())
                            .unwrap()
                            .destination(tcp.source())
                            .unwrap()
                            .payload(pct.payload())
                            .unwrap()
                            .flags(pct.flags())
                            .unwrap()
                            .build()
                            .unwrap();

                        dev.write(&response[..response.len()]).unwrap();
                    }
                }*/
                _ => info!("other"),
            }
        }
    }
}
