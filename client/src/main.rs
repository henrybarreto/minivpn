use std::{
    env,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, TcpStream},
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
};

extern crate tun;

fn main() {
    let mut config = tun::Configuration::default();
    let args: Vec<String> = env::args().collect();

    let mut address = String::from_str("10.0.0.").unwrap();
    address.push_str(&args[1]);

    let addr = IpAddr::from_str(&address).unwrap();

    config
        .address(Ipv4Addr::from_str(&address).unwrap())
        .netmask((255, 255, 255, 0))
        .up();

    let dev = tun::create(&config).unwrap();
    dev.set_nonblock().unwrap();

    let address = &args[2];
    let mut socket = TcpStream::connect(address).expect("couldn't bind to address");
    socket
        .set_nonblocking(true)
        .expect("set_nonblocking call failed");
    socket.write(&bincode::serialize(&addr).unwrap()).unwrap();

    let mdev = Arc::new(Mutex::new(dev));
    let msocket = Arc::new(Mutex::new(socket));

    let csocket = msocket.clone();
    let cdev = mdev.clone();
    thread::spawn(move || loop {
        let mut buffer = [0; 4096];

        let mut dev = cdev.lock().unwrap();
        let read: usize = match dev.read(&mut buffer) {
            Ok(read) => read,
            Err(_) => {
                continue;
            }
        };
        drop(dev);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            let mut socket = csocket.lock().unwrap();
            socket
                .write(&buffer[..read])
                .expect("couldn't send message");
            drop(socket);

            dbg!(ip);
        } else {
            dbg!("PACKET NOT IP");
        }
    });

    let csocket = msocket.clone();
    let cdev = mdev.clone();
    thread::spawn(move || loop {
        let mut buffer = [0; 4096];

        let mut socket = csocket.lock().unwrap();
        let read = match socket.read(&mut buffer) {
            Ok(read) => {
                if read == 0 {
                    break;
                }

                read
            }
            Err(_) => {
                continue;
            }
        };
        drop(socket);

        if let Ok(ip) = packet::ip::v4::Packet::new(&buffer[..read]) {
            let mut dev = cdev.lock().unwrap();
            match dev.write(&buffer[..read]) {
                Ok(_) => {
                    dbg!(ip);
                }
                Err(_) => {
                    continue;
                }
            }
            drop(dev);
        } else {
            dbg!("PACKET NOT IP");
        }
    });

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
