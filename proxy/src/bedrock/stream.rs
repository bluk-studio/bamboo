use crate::{packet::Packet, StreamReader, StreamWriter};
use common::version::ProtocolVersion;
use std::{
  io,
  net::{SocketAddr, UdpSocket},
  sync::{mpsc::Receiver, Arc},
};

pub struct BedrockStreamReader {
  rx: Receiver<Vec<u8>>,
}

pub struct BedrockStreamWriter {
  sock: Arc<UdpSocket>,
  addr: SocketAddr,
}

impl BedrockStreamReader {
  pub fn new(rx: Receiver<Vec<u8>>) -> Self {
    BedrockStreamReader { rx }
  }
}

impl BedrockStreamWriter {
  pub fn new(sock: Arc<UdpSocket>, addr: SocketAddr) -> Self {
    BedrockStreamWriter { sock, addr }
  }
}

#[async_trait]
impl StreamWriter for BedrockStreamWriter {
  async fn write(&mut self, packet: Packet) -> io::Result<()> {
    Ok(())
  }
}
#[async_trait]
impl StreamReader for BedrockStreamReader {
  fn read(&mut self, ver: ProtocolVersion) -> io::Result<Option<Packet>> {
    dbg!("{:?}", self.rx.recv().unwrap());
    Ok(None)
  }
}
