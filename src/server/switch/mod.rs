use tokio::net::UdpSocket;

use log::{info, trace};

mod worker;
use worker::worker;

use crate::server::entities::{Address, Peers};

/// Switch server address.
const SWITCH_SERVER: &str = "0.0.0.0";
/// Switch port address.
const SWITCH_PORT: u16 = 9807;

/// start the Obirt switch.
///
/// The switch redirects the IP packages to its destinations, if it exist.
pub async fn start(peers: &impl Peers) {
    info!("Initing Obirt switch");

    let address = Address::new(SWITCH_SERVER, SWITCH_PORT);

    let socket = UdpSocket::bind(address.to_string()).await.unwrap();
    info!("Obirt switch listening for packets on {}", address.port);

    trace!("Starting workers");

    tokio::join!(
        worker(0, &socket, peers),
        worker(1, &socket, peers),
        worker(2, &socket, peers),
        worker(3, &socket, peers),
        worker(4, &socket, peers),
        worker(5, &socket, peers),
        worker(6, &socket, peers),
        worker(7, &socket, peers),
    );

    info!("Stopping Obirt switch")
}
