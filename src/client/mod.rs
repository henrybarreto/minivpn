use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

pub mod connect;
pub mod data;
pub mod io;
pub mod loader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IP {
    pub source: Ipv4Addr,
    pub destination: Ipv4Addr,
    pub data: Vec<u8>,
}
