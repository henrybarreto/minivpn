use tokio::net::UdpSocket;

use log::{info, trace};

mod worker;
use worker::worker;

use crate::server::entities::{Address, Peers};

const ROUTER_SERVER: &str = "0.0.0.0";
const ROUTER_PORT: u16 = 9807;

/// start the Obirt router.
///
/// The router redirects the IP packages to its destinations, if it exist.
pub async fn start(peers: &impl Peers) {
    info!("Initing Obirt router");

    let address = Address::new(ROUTER_SERVER, ROUTER_PORT);

    let socket = UdpSocket::bind(address.to_string()).await.unwrap();
    info!("Obirt router listening for packets on {}", address.port);

    trace!("Starting routers workers");

    // TODO: create workers based on CPU.
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

    info!("Stopping Obirt router")
}
