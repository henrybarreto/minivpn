use socket2;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream, UdpSocket};
use std::thread;
use std::time::Duration;

fn main() {
    /*let v = 0x0300;
    let protocol: Option<socket2::Protocol> = Some(socket2::Protocol::from(v));*/

    // socket.bind(&address.into()).unwrap();
    // socket.listen(128).unwrap();

    println!("TCP server listening on");

    let protocol: Option<socket2::Protocol> = Some(socket2::Protocol::from(6));
    let mut socket =
        socket2::Socket::new_raw(socket2::Domain::IPV4, socket2::Type::RAW, protocol).unwrap();

    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000).into();

    socket.bind(&address).unwrap();

    let mut buffer = [0u8; 1024];

    let protocol_to: Option<socket2::Protocol> = Some(socket2::Protocol::from(6));
    let address_to: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8001).into();
    let mut socket_to =
        socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::RAW, protocol_to).unwrap();

    socket_to.connect(&address_to).unwrap();

    let sent = socket_to
        .send(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
        .unwrap();
    println!("sent {:?}", sent);

    unsafe {
        let mut resp_buf_u: [MaybeUninit<u8>; 4096] = MaybeUninit::uninit().assume_init();

        let rcv = socket.recv(&mut resp_buf_u).unwrap();
        println!("Received {} bytes from the client", rcv);

        socket_to
            .send(b"iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii")
            .unwrap();

        let recved = socket_to.recv(&mut resp_buf_u).unwrap();
        println!("recved {:?}", recved);

        let resp_buf: &[u8] = std::mem::transmute(&resp_buf_u[0..50]);
        println!("Received: {:?}", resp_buf);
        // to string
        // println!("Received: {:?}", std::str::from_utf8(resp_buf).unwrap());
    }

    // ------

    /*let protocol: Option<socket2::Protocol> = Some(socket2::Protocol::from(6));
    let mut socket =
        socket2::Socket::new_raw(socket2::Domain::IPV4, socket2::Type::RAW, protocol).unwrap();

    let address: socket2::SockAddr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000).into();*/

    /*loop {
        unsafe {
            println!("Waiting for a client to connect...");
            // let mut resp_buf_u: [MaybeUninit<u8>; 4096] = MaybeUninit::uninit().assume_init();
            // check it is http
            let rcv = socket.read(&mut buffer).unwrap();
            println!("Data: {:?}", &buffer[0..rcv]);
            if buffer[12] == 127 && buffer[13] == 0 && buffer[14] == 0 && buffer[15] == 1 {
                // thread::sleep(Duration::from_secs(10));
                println!("Received from localhost");
                println!("Received {} bytes from the client", rcv);
                // show protocol
                println!("Protocol: {:?}", buffer[9]);

                //socket_to.send_to(&buffer[0..rcv], &address).unwrap();
                let sent = socket_to.write(&buffer[0..rcv]).unwrap();
                println!("sent {:?}", sent);
                thread::sleep(Duration::from_secs(1));
            }

            // let resp_buf: &[u8] = std::mem::transmute(&resp_buf_u[..rcv]);
            // println!("Received: {:?}", resp_buf);
            /*let (sock, addr) = socket.accept().unwrap();
            let mut resp_buf_u: [MaybeUninit<u8>; 4096] = MaybeUninit::uninit().assume_init();
            let rcv = sock.recv(&mut resp_buf_u).unwrap();
            println!("Received {} bytes from the client", rcv);

            let resp_buf: &[u8] = std::mem::transmute(&resp_buf_u[..rcv]);
            println!("Received: {:?}", resp_buf);
            // to strign
            println!("Received: {:?}", std::str::from_utf8(resp_buf).unwrap());

            let local_socket =
                socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None).unwrap();

            let local_address: socket2::SockAddr =
                SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8000).into();

            local_socket.connect(&local_address.into()).unwrap();

            let sent = local_socket.send(&resp_buf[0..rcv]).unwrap();
            println!("sent to server {:?}", sent);*/
        }
    }*/
}
