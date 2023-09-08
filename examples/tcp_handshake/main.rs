use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, SocketAddrV4};

use packet::{AsPacket, Builder, Packet};
use socket2::Socket;

fn main() {
    let mut socketA = Socket::new_raw(
        socket2::Domain::IPV4,
        socket2::Type::RAW,
        Some(socket2::Protocol::TCP),
    )
    .unwrap();

    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(172, 17, 0, 2), 0).into();

    socketA.set_header_included(true).unwrap();
    socketA.connect(&address).unwrap();

    let a = packet::ip::v4::Builder::default()
        .id(0x01)
        .unwrap()
        .ttl(64)
        .unwrap()
        .source(Ipv4Addr::new(172, 17, 0, 1)) // GATEWAY
        .unwrap()
        .destination(Ipv4Addr::new(172, 17, 0, 2))
        .unwrap()
        .protocol(packet::ip::Protocol::Tcp)
        .unwrap()
        .tcp()
        .unwrap()
        .source(7070)
        .unwrap()
        .destination(8080)
        .unwrap()
        .sequence(1)
        //.unwrap()
        //.acknowledgment(2)
        .unwrap()
        .window(1024)
        .unwrap()
        .flags(packet::tcp::Flags::SYN)
        .unwrap()
        .build()
        .unwrap();

    let aip = packet::ip::v4::Packet::new(a).unwrap();
    let atcp = packet::tcp::Packet::new(aip.payload()).unwrap();

    println!("{:?}", aip);
    println!("REQUEST {:?}", atcp);

    let wrote = socketA.write(&aip.as_ref()).unwrap();
    println!("wrote {} bytes", wrote);

    let mut buf = [0; 1024];
    let read = socketA.read(&mut buf).unwrap();

    let b = packet::ip::v4::Packet::new(&buf[..read]).unwrap();
    let btcp = packet::tcp::Packet::new(b.payload()).unwrap();

    println!("RESPONSE {:?}", btcp);

    println!("--------->");

    let c = packet::ip::v4::Builder::default()
        .id(0x01)
        .unwrap()
        .ttl(64)
        .unwrap()
        .source(Ipv4Addr::new(172, 17, 0, 1)) // GATEWAY
        .unwrap()
        .destination(Ipv4Addr::new(172, 17, 0, 2))
        .unwrap()
        .protocol(packet::ip::Protocol::Tcp)
        .unwrap()
        .tcp()
        .unwrap()
        .source(7070)
        .unwrap()
        .destination(8080)
        .unwrap()
        .sequence(2)
        .unwrap()
        .acknowledgment(btcp.sequence() + 1)
        .unwrap()
        .window(1024)
        .unwrap()
        .flags(packet::tcp::Flags::ACK)
        .unwrap()
        .build()
        .unwrap();

    let cip = packet::ip::v4::Packet::new(c).unwrap();
    let ctcp = packet::tcp::Packet::new(cip.payload()).unwrap();

    println!("{:?}", cip);
    println!("REQUEST {:?}", ctcp);

    let wrote = socketA.write(&cip.as_ref()).unwrap();
    println!("wrote {} bytes", wrote);

    let mut buf = [0; 1024];
    let read = socketA.read(&mut buf).unwrap();

    let d = packet::ip::v4::Packet::new(&buf[..read]).unwrap();

    let dtcp = packet::tcp::Packet::new(d.payload()).unwrap();

    println!(">>> {:?}", d);
    println!("RESPONSE {:?}", dtcp);
}
