use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc::Sender, Mutex};
use tonic::{Status, Streaming};

use common::{
  math::{Pos, UUID},
  net::{cb, sb},
  proto,
  util::chat::{Chat, Color, HoverEvent},
};

use crate::{block, item, player::Player};

pub struct Connection {
  rx:     Mutex<Streaming<proto::Packet>>,
  tx:     Mutex<Sender<Result<proto::Packet, Status>>>,
  closed: AtomicBool,
}

impl Connection {
  pub(crate) fn new(
    rx: Streaming<proto::Packet>,
    tx: Sender<Result<proto::Packet, Status>>,
  ) -> Self {
    Connection { rx: Mutex::new(rx), tx: Mutex::new(tx), closed: false.into() }
  }

  /// This waits for the a login packet from the proxy. If any other packet is
  /// recieved, this will panic. This should only be called right after a
  /// connection is created.
  pub(crate) async fn wait_for_login(&self) -> (String, UUID) {
    let p = match self.rx.lock().await.message().await.unwrap() {
      Some(p) => sb::Packet::from_proto(p),
      None => panic!("connection was closed while listening for a login packet"),
    };
    match p.id() {
      sb::ID::Login => (p.get_str("username").into(), p.get_uuid("uuid")),
      _ => panic!("expecting login packet, got: {}", p),
    }
  }

  /// This starts up the recieving loop for this connection. Do not call this
  /// more than once.
  pub(crate) async fn run(&self, player: &Mutex<Player>) -> Result<(), Status> {
    'running: loop {
      let p = match self.rx.lock().await.message().await? {
        Some(p) => sb::Packet::from_proto(p),
        None => break 'running,
      };
      match p.id() {
        sb::ID::Chat => {
          let message = p.get_str("message");
          let world;
          let username;
          {
            let p = player.lock().await;
            world = p.clone_world();
            username = p.username().to_string();
          }
          let mut msg = Chat::empty();
          msg.add("<".into());
          msg.add(username).color(Color::Red);
          msg.add("> ".into());
          msg.add(message.into()).on_hover(HoverEvent::ShowText("Hover time".into()));
          world.broadcast(&msg).await;
        }
        sb::ID::SetCreativeSlot => {
          let slot = p.get_short("slot");
          let id = p.get_int("item-id");
          let count = p.get_byte("item-count");
          let _nbt = p.get_byte_arr("item-nbt");

          let mut p = player.lock().await;
          let id = p.world().get_item_converter().to_latest(id as u32, p.ver().block());
          let inv = p.inventory_mut();
          inv.set(slot as u32, item::Stack::new(item::Type::from_u32(id)).with_amount(count));
        }
        sb::ID::BlockDig => {
          let pos = p.get_pos("location");
          let world = player.lock().await.clone_world();
          world.set_kind(pos, block::Kind::Air).await.unwrap();
        }
        sb::ID::BlockPlace => {
          let mut pos = p.get_pos("location");
          let dir = p.get_byte("direction");

          if pos == Pos::new(-1, -1, -1) && dir as i8 == -1 {
            // Client is eating, or head is inside block
          } else {
            pos += Pos::dir_from_byte(dir);
            let world = player.lock().await.clone_world();
            world.set_kind(pos, block::Kind::Stone).await.unwrap();
          }
        }
        sb::ID::Position => {
          let mut player = player.lock().await;
          player.set_next_pos(p.get_double("x"), p.get_double("y"), p.get_double("z"));
        }
        sb::ID::PositionLook => {
          let mut player = player.lock().await;
          player.set_next_pos(p.get_double("x"), p.get_double("y"), p.get_double("z"));
          player.set_next_look(p.get_float("yaw"), p.get_float("pitch"));
        }
        sb::ID::Look => {
          let mut player = player.lock().await;
          player.set_next_look(p.get_float("yaw"), p.get_float("pitch"));
        }
        // _ => warn!("got unknown packet from client: {:?}", p),
        _ => (),
      }
      // info!("got packet from client {:?}", p);
    }
    self.closed.store(true, Ordering::SeqCst);
    Ok(())
  }

  /// Sends a packet to the proxy, which will then get sent to the client.
  pub async fn send(&self, p: cb::Packet) {
    self.tx.lock().await.send(Ok(p.into_proto())).await.unwrap();
  }

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}
