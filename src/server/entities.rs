use std::{
    collections::HashMap,
    future::Future,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub struct Address {
    pub server: String,
    pub port: u16,
}

impl Address {
    pub fn new(server: &str, port: u16) -> Self {
        return Address {
            server: String::from(server),
            port,
        };
    }
}

impl ToString for Address {
    fn to_string(&self) -> String {
        return format!("{}:{}", self.server, self.port);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub addr: SocketAddr,
}

/// A trait for create ways of managing Peers.
pub trait Peers {
    /// set inserts a Peer for an IP.
    fn set(&self, ip: Ipv4Addr, peer: Peer) -> impl Future<Output = Option<Peer>> + Send;
    /// get a Peer from an IP.
    fn get(&self, ip: Ipv4Addr) -> impl Future<Output = Option<Peer>> + Send;
}

/// In memory Peer's manager.
pub struct MemPeers {
    peers: Arc<RwLock<HashMap<Ipv4Addr, Peer>>>,
}

impl Peers for MemPeers {
    async fn set(&self, ip: Ipv4Addr, peer: Peer) -> Option<Peer> {
        let mut writer = self.peers.write().await;

        return writer.insert(ip, peer);
    }

    async fn get(&self, ip: Ipv4Addr) -> Option<Peer> {
        let reader = self.peers.read().await;
        let got = reader.get(&ip)?;

        return Some(got.clone());
    }
}

impl Default for MemPeers {
    fn default() -> Self {
        return MemPeers {
            peers: Arc::new(RwLock::new(HashMap::<Ipv4Addr, Peer>::new())),
        };
    }
}
