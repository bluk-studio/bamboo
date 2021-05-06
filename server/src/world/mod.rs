mod chunk;

use std::{
  collections::HashMap,
  future::Future,
  sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::{
  sync::{mpsc::Sender, Mutex, MutexGuard, RwLock, RwLockReadGuard},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, UUID},
  net::cb,
  proto::Packet,
  version::ProtocolVersion,
};

use crate::{net::Connection, player::Player};
use chunk::MultiChunk;

pub struct World {
  chunks:  RwLock<HashMap<ChunkPos, Mutex<MultiChunk>>>,
  players: Mutex<Vec<Arc<Mutex<Player>>>>,
  eid:     Arc<AtomicU32>,
}

#[derive(Clone)]
pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds: Vec<Arc<World>>,
}

impl World {
  pub fn new() -> Self {
    World {
      chunks:  RwLock::new(HashMap::new()),
      players: Mutex::new(vec![]),
      eid:     Arc::new(1.into()),
    }
  }
  async fn new_player(self: Arc<Self>, conn: Arc<Connection>, player: Player) {
    let player = Arc::new(Mutex::new(player));
    self.players.lock().await.push(player.clone());

    let c = conn.clone();
    tokio::spawn(async move {
      // Network recieving task
      c.run().await.unwrap();
    });

    let mut int = time::interval(Duration::from_millis(50));
    tokio::spawn(async move {
      // Player tick loop
      let mut tick = 0;
      loop {
        int.tick().await;
        let p = player.lock().await;
        // Do player collision and packets and stuff
        info!("player tick for {}", p.username());
        if p.conn().closed() {
          break;
        }
        // Once per second, send keep alive packet
        if tick % 20 == 0 {
          let mut out = cb::Packet::new(cb::ID::KeepAlive);
          out.set_i32(0, 1234556);
          conn.send(out).await;
        }
        for x in -10..10 {
          for z in -10..10 {
            let chunks = self.chunks().await;
            let chunk = chunks[&ChunkPos::new(x, z)].lock().await;

            let mut out = cb::Packet::new(cb::ID::ChunkData);
            out.set_other(&chunk.to_proto(p.ver().block())).unwrap();
            conn.send(out).await;
          }
        }
        tick += 1;
      }
    });
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> u32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }

  /// Returns a locked reference to all the chunks in the world.
  pub async fn chunks<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<ChunkPos, Mutex<MultiChunk>>> {
    self.chunks.read().await
  }
  // Returns a locked Chunk. This will generate a new chunk if there is not one
  // stored there.
  // pub async fn chunk<'a>(&'a self, pos: ChunkPos) -> MutexGuard<'a, MultiChunk>
  // {   if !self.chunks.read().await.contains_key(&pos) {
  //     // TODO: Terrain generation goes here
  //     self.chunks.write().await.insert(pos, Mutex::new(MultiChunk::new()));
  //   }
  //   // self.chunks.read().await[&pos].lock().await
  // }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager { worlds: vec![Arc::new(World::new())] }
  }

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    // Default world. Might want to change this later, but for now this is easiest.
    // TODO: Player name, uuid
    let conn = Arc::new(Connection::new(req, tx));
    let w = self.worlds[0].clone();
    let player = Player::new(
      w.eid(),
      "macmv".into(),
      UUID::from_u128(0x1111111),
      conn.clone(),
      ProtocolVersion::V1_8,
    );
    w.new_player(conn, player).await;
  }
}
