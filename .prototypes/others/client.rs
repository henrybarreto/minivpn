use packet::{self, Builder, Packet};
use socket2;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream};
use std::time::Duration;
use std::{thread, time};
use tun::Device;

fn main() {
    let mut config = tun::Configuration::default();
    config
        .name("minivpn0")
        .address((10, 0, 0, 1))
        .netmask((255, 255, 255, 0))
        .up();

    let mut socket = socket2::Socket::new_raw(
        socket2::Domain::IPV4,
        socket2::Type::RAW,
        Some(socket2::Protocol::TCP),
    )
    .unwrap();

    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000).into();
    socket.connect(&address).unwrap();

    let mut dev = tun::create(&config).unwrap();
    let mut buf = [0; 4096];

    loop {
        let mut read = dev.read(&mut buf).unwrap();
        // string
        /*println!(
            "payload {:?}",
            String::from_utf8_lossy(&buf[..read]).to_string()
        );*/
        /*if buf[9] != 6 {
            continue;
        }*/

        if buf[9] != 6 {
            continue;
        }

        println!("payload {:?}", &buf[..read]);
        println!("-------------");
        let ip = packet::ip::v4::Packet::new(&buf[..read]).unwrap();
        println!("ip {:?}", ip);
        println!("-------------");
        let tcp = packet::tcp::Packet::new(ip.payload()).unwrap();
        println!("tcp {:?}", tcp);
        println!("-------------");

        // let ip_source = ip.source();
        // let ip_destination = ip.destination();

        // let port_source = tcp.source();
        // let port_destination = tcp.destination();

        /*let mut payload_ref = tcp.payload();
        let mut payload = [0; 4096];
        for i in 0..payload_ref.len() {
            payload[i] = payload_ref[i];
        }*/

        /*let mut builder = packet::ip::v4::Builder::default();
        builder.source(Ipv4Addr::new(127, 0, 0, 1));
        builder.destination(Ipv4Addr::new(127, 0, 0, 1));
        builder.payload(&payload[..payload.len()]);*/

        // .destination(Ipv4Addr::new(127, 0, 0, 1))
        // .payload(&payload[..payload.len()]);
        // let builder = packet::tcp::Builder::default().source(8000).destination(8000).payload(&payload[..payload.len()]);
        //println!("ip {:?}", ip);
        //println!("tcp {:?}", tcp);

        // source (same)
        // payload[12] = 10;
        // payload[13] = 0;
        // payload[14] = 0;
        // payload[15] = 1;

        // let mut builder = packet::ip::v4::Builder::default();

        // builder = builder.payload(&payload[..payload.len()]).unwrap();
        // builder = builder.source(Ipv4Addr::new(10, 0, 0, 1)).unwrap();
        // builder = builder.destination(Ipv4Addr::new(127, 0, 0, 1)).unwrap();

        // let built = builder.build().unwrap();
        //

        //let mut builder = packet::tcp::Builder::default();

        //builder = builder.source(tcp.destination()).unwrap();
        //builder = builder.destination(tcp.source()).unwrap();

        //let built = builder.build().unwrap();

        // // destination
        // payload[16] = 127; // 10
        // payload[17] = 0; // 0
        // payload[18] = 0; // 0
        // payload[19] = 1; // 100

        // print payload
        //println!("payload {:?}", &payload[..payload.len()]);

        //let mut wrote = socket.write(&built[..read]).unwrap();
        println!("payload to client{:?}", ip.payload());
        let mut wrote = socket.write(ip.payload()).unwrap();
        println!("wrote to client {:?}", wrote);

        println!("-------------");

        println!("reading from client");
        let mut buff = [0; 4096];
        read = socket.read(&mut buff).unwrap();
        println!("read from client {:?}", read);
        println!("------------>");

        println!("buff {:?}", &buff[..read]);

        println!("------------>");

        //let pi = packet::ip::v4::Packet::new(&buff[..read]).unwrap();

        // let mut builder = packet::ip::v4::Builder::default();

        // builder = builder.source(Ipv4Addr::new(10, 0, 0, 100)).unwrap();
        // builder = builder.destination(Ipv4Addr::new(10, 0, 0, 1)).unwrap();
        // builder = builder.payload(&buff[20..read]).unwrap();

        // let built = builder.build().unwrap();

        // println!("built {:?}", built);

        println!("------------>");

        // let pi = packet::ip::v4::Packet::new(&buff[..read]).unwrap();
        // println!("pi {:?}", pi);
        // buff[12] = 10;
        // buff[13] = 0;
        // buff[14] = 0;
        // buff[15] = 1;

        // buff[16] = 127;
        // buff[17] = 0;
        // buff[18] = 0;
        // buff[19] = 1;

        println!("------------#");
        let mut pi = packet::ip::v4::Packet::new(&buff[..read]).unwrap();
        println!("pi {:?}", pi);
        println!("------------#");

        let mut clone = packet::ip::v4::Builder::default();
        clone = clone.source(Ipv4Addr::new(10, 0, 0, 100)).unwrap();
        clone = clone.destination(Ipv4Addr::new(10, 0, 0, 1)).unwrap();
        clone = clone.id(pi.id()).unwrap();
        clone = clone.flags(pi.flags()).unwrap();
        clone = clone.ttl(pi.ttl()).unwrap();
        clone = clone.protocol(pi.protocol()).unwrap();
        clone = clone.offset(pi.offset()).unwrap();

        let mut built = clone.build().unwrap();

        let payload = pi.payload().iter().cloned().collect::<Vec<u8>>();

        built.append(&mut payload.clone());

        let parsed = packet::ip::v4::Packet::new(&built[..built.len()]).unwrap();
        println!("------------@");
        println!("payload {:?}", payload);
        println!("cloned {:?}", parsed);
        println!("buffer {:?}", built);
        println!("------------@");

        let pct = packet::tcp::Packet::new(&payload[..payload.len()]).unwrap();
        println!("------------<<<");
        println!("tcp {:?}", pct);
        println!("------------<<<");

        // let mut builder = packet::ip::v4::Builder::default();

        // builder = builder.source(Ipv4Addr::new(10, 0, 0, 100)).unwrap();
        // builder = builder.destination(Ipv4Addr::new(10, 0, 0, 1)).unwrap();
        // builder = builder.protocol(pi.protocol()).unwrap();
        // builder = builder.payload(pi.payload()).unwrap();
        // // id and flags
        // builder = builder.id(pi.id()).unwrap();
        // builder = builder.flags(pi.flags()).unwrap();

        // let built = builder.build().unwrap();

        // println!("-------------");

        // println!("built {:?}", built);

        println!("-------------");

        //let parsed = packet::ip::v4::Packet::new(&built[..read]).unwrap();

        //println!("parsed {:?}", parsed);
        //

        wrote = dev.write(&built[..built.len()]).unwrap(); // <<<<<<<<<<<<
        println!("wrote to dev {:?}", wrote);

        println!("-------------");

        // thread::sleep(Duration::from_millis(10000));

        read = dev.read(&mut buf).unwrap();
        println!("read second time {:?}", read);

        println!("1------------");

        let ip = packet::ip::v4::Packet::new(&buf[..read]).unwrap();
        println!("ip {:?}", ip);

        println!("1------------");

        let tcp = packet::tcp::Packet::new(ip.payload()).unwrap();

        println!("tcp {:?}", tcp);

        println!("1------------");

        println!("payload {:?}", &buf[..read]);

        println!("1------------");

        return;

        wrote = socket.write(ip.payload()).unwrap();
        println!("wrote to client {:?}", wrote);

        println!("1------------");

        read = socket.read(&mut buff).unwrap();
        println!("read from client {:?}", read);

        let pi = packet::ip::v4::Packet::new(&buff[..read]).unwrap();

        println!("pi {:?}", pi);

        println!("1------------");

        wrote = dev.write(&buff[0..read]).unwrap();
        println!("wrote to dev {:?}", wrote);

        println!("2------------");

        read = dev.read(&mut buf).unwrap();
        println!("read third time {:?}", read);

        continue;
        // buff[12] = 10;
        // buff[13] = 0;
        // buff[14] = 0;
        // buff[15] = 100;

        // buff[16] = 10;
        // buff[17] = 0;
        // buff[18] = 0;
        // buff[19] = 1;

        //let mut builder = packet::tcp::Builder::default();

        //builder = builder.source(tcp.destination()).unwrap();
        //builder = builder.destination(tcp.source()).unwrap();
        //builder = builder.payload(&buff[..read]).unwrap();

        //let built = builder.build().unwrap();

        //println!("{:?}", built);

        // println!("read {:?}", read);

        // buff[16] = 10;
        // buff[17] = 0;
        // buff[18] = 0;
        // buff[19] = 1;

        // println!("buffer {:?}", &buff[0..read]);

        // ----
        /*read = dev.read(&mut buf).unwrap();

        println!("read second time {:?}", read);

        socket.write(&buf[0..read]).unwrap();

        println!("write second time {:?}", read);

        read = socket.read(&mut buff).unwrap();

        println!("read third time {:?}", read);

        let p = packet::tcp::Packet::new(&buff[0..read]).unwrap();
        println!("tcp {:?}", p);

        dev.write(p.payload()).unwrap();

        println!("write third time {:?}", read);*/

        /*unsafe {
            let mut resp_buf = [0 as u8; 4096];
            let mut resp_buf_u: [MaybeUninit<u8>; 4096] = MaybeUninit::uninit().assume_init();

            let resp = socket.recv(&mut resp_buf_u).unwrap();
            println!("resp {:?}", resp);
        }*/

        /*let protocol: Protocols = Protocols::from_package(&buf[0..read]);

        println!("read {:?}", read);
        println!("{:?}", &buf[0..read]);

        let mut socket = socket2::Socket::new_raw(
            socket2::Domain::IPV4,
            socket2::Type::RAW,
            Some(socket2::Protocol::from(6)),
        )
        .unwrap();

        println!("{:?}", socket.local_addr().unwrap());
        let address: socket2::SockAddr =
            SocketAddrV4::new(Ipv4Addr::new(104, 248, 33, 94), 8000).into();

        socket.connect(&address).unwrap();

        let sent = socket.send_to(&buf[..read], &address).unwrap();
        println!("sent {:?}", sent);

        unsafe {
            let mut resp_buf = [0 as u8; 4096];
            let mut resp_buf_u: [MaybeUninit<u8>; 4096] = MaybeUninit::uninit().assume_init();

            let resp = socket.recv(&mut resp_buf_u).unwrap();

            resp_buf_u.iter().enumerate().for_each(|(i, v)| {
                resp_buf[i] = unsafe { v.assume_init() };
            });

            println!("recv");

            println!("write");
            println!("{:?}", &resp_buf[0..resp]);

            dev.write(&resp_buf[0..resp]).unwrap();
        }

        match protocol {
            Protocols::TCP => {
                let mut socket = socket2::Socket::new_raw(
                    socket2::Domain::IPV4,
                    socket2::Type::RAW,
                    Some(socket2::Protocol::from(6)),
                )
                .unwrap();

                let address: socket2::SockAddr =
                    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000).into();

                println!("{:?}", address);

                let sent = socket.send_to(&buf[0..read], &address).unwrap();
                println!("sent {:?}", sent);

                // dev.send(&buf[0..read]).unwrap();
            }
            Protocols::UDP => {}
            _ => continue,
        }*/
    }

    /*let socket = socket2::Socket::new_raw(
        socket2::Domain::IPV4,
        socket2::Type::RAW,
        Some(socket2::Protocol::TCP),
    )
    .unwrap();

    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000).into();

    println!("{:?}", address);
    socket.connect(&address).unwrap();

    loop {
        let read = dev.read(&mut buf).unwrap();

        let a = headers::IP::from_package(&buf[0..read]);
        if a.destination == Ipv4Addr::new(10, 0, 0, 125) {
            unsafe {
                println!("{:?}", socket.local_addr().unwrap());
                socket.send(&buf[0..read]).unwrap();

                println!("sent");
                let mut resp_buf = [0 as u8; 4096];
                let mut resp_buf_u: [MaybeUninit<u8>; 4096] = MaybeUninit::uninit().assume_init();

                let resp = socket.recv(&mut resp_buf_u).unwrap();

                resp_buf_u.iter().enumerate().for_each(|(i, v)| {
                    resp_buf[i] = unsafe { v.assume_init() };
                });

                println!("recv");
                println!("{:?}", resp_buf);

                // let as_string = std::str::from_utf8(&resp_buf[0..resp]).unwrap();
                // println!("{:?}", as_string);
                // overrite the source and destination ip

                println!("write");
                println!("{:?}", &resp_buf[0..resp]);
                dev.write(&resp_buf[0..resp]).unwrap();
            }
        }

        /*socket.bind(&address).unwrap();
        socket.listen(128).unwrap();*/

        //println!("{:?}", &buf[0..amount]);

        /*let ping = headers::IP::from_package(&buf[0..read]);
        println!("ping {:#?}", ping);*/

        /*let pong = headers::IP::new(ping.destination, ping.source);
        println!("pong");
        println!("source {:?}", pong.source);
        println!("destination {:?}", pong.destination);

        dev.write(&buf[0..read]).unwrap();*/
    }*/
}

// 16-20 is ip source
// println!("{:?}", &buf[16..20]);
// println!("{:?}", string::String::from_utf8_lossy(&buf[14..20]));
// 20-24 is ip destination
// println!("{:?}", &buf[20..24]);
// println!("{:?}", string::String::from_utf8_lossy(&buf[20..24]));
// length
// println!("{:?}", &buf[0..amount]);
// 10.0.0.125
/*if buf[16] == 10 && buf[17] == 0 && buf[18] == 0 && buf[19] == 125 {
    println!("{:?}", &buf[0..amount]);
    // remove ip header
    let rest = &buf[20..amount];
    println!("{:?}", &rest);
    // println!("{:?}", string::String::from_utf8_lossy(&buf[0..amount]));
    /*println!("{:?}", &buf[20..24]);
    // println!("{:?}", string::String::from_utf8_lossy(&buf[20..24]));
    //
    println!("{:?}", &buf[0..amount]);

    // println!("{:?}", string::String::from_utf8_lossy(&buf[0..amount]));
    // len
    println!("{:?}", &buf[0..amount].len());

    // redirect package to google dot com
    let mut stream = std::net::TcpStream::connect("127.0.0.1:8000").unwrap();
    //write body only
    stream.write(&buf).unwrap();

    let mut response = [0; 4096];
    let readed = stream.read(&mut response).unwrap();

    //println!("{:?}", string::String::from_utf8_lossy(&response));

    dev.write(&response[0..readed]).unwrap();*/
}*/
