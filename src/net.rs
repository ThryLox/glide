use std::net::SocketAddr;
use tokio::net::UdpSocket;
use anyhow::Result;
use tracing::info;
use crate::protocol::InputEvent;

pub struct NetworkEngine {
    socket: UdpSocket,
}

impl NetworkEngine {
    pub async fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        info!("Glide-KVM UDP Engine bound on {}", addr);
        Ok(Self { socket })
    }

    pub async fn send_event(&self, event: &InputEvent, target: SocketAddr) -> Result<()> {
        let bytes = bincode::serialize(event)?;
        self.socket.send_to(&bytes, target).await?;
        Ok(())
    }

    pub async fn receive_event(&self) -> Result<(InputEvent, SocketAddr)> {
        let mut buf = [0u8; 1024];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        let event: InputEvent = bincode::deserialize(&buf[..len])?;
        Ok((event, addr))
    }
}
