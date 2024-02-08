use tokio::net::UdpSocket;

use log::{info, trace};

mod worker;

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

    trace!("Starting routing worker");

    worker::init(&socket, peers).await;

    info!("Obirt router stopped")
}
