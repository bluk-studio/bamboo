use super::{BiomeGen, WorldGen};
use crate::block;
use crate::world::chunk::MultiChunk;
use common::math::{ChunkPos, PointGrid, Pos, Voronoi};
use noise::{NoiseFn, Perlin};
use std::cmp::Ordering;

pub struct Gen {
  cacti: PointGrid,
}

impl Gen {
  pub fn new() -> Box<dyn BiomeGen + Send> {
    Box::new(Self { cacti: PointGrid::new(12345, 16, 10) })
  }
}

impl BiomeGen for Gen {
  fn fill_chunk(&self, world: &WorldGen, pos: ChunkPos, c: &mut MultiChunk) {
    // This is the height at the middle of the chunk. It is a good average height
    // for the whole chunk.
    let average_stone_height = world.height_at(pos.block() + Pos::new(8, 0, 8)) as i32 - 5;
    c.fill_kind(Pos::new(0, 0, 0), Pos::new(15, average_stone_height, 15), block::Kind::Stone)
      .unwrap();
    for x in 0..16 {
      for z in 0..16 {
        let height = world.height_at(pos.block() + Pos::new(x, 0, z)) as i32;
        let stone_height = height - 5;
        match stone_height.cmp(&average_stone_height) {
          Ordering::Less => {
            c.fill_kind(
              Pos::new(x, stone_height + 1, z),
              Pos::new(x, average_stone_height, z),
              block::Kind::Air,
            )
            .unwrap();
          }
          Ordering::Greater => {
            c.fill_kind(
              Pos::new(x, average_stone_height, z),
              Pos::new(x, stone_height, z),
              block::Kind::Stone,
            )
            .unwrap();
          }
          _ => {}
        }
        c.fill_kind(Pos::new(x, height - 4, z), Pos::new(x, height - 1, z), block::Kind::Sandstone)
          .unwrap();
        c.set_kind(Pos::new(x, height, z), block::Kind::Sand).unwrap();
        // Trees
        let p = pos.block() + Pos::new(x, 0, z);
        if self.cacti.contains(p.x(), p.z()) {
          c.fill_kind(Pos::new(x, height + 1, z), Pos::new(x, height + 4, z), block::Kind::Cactus)
            .unwrap();
        }
      }
    }
  }
}
