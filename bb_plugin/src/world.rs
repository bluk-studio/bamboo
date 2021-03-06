use crate::{block, particle::Particle, player::Player, FromFfi, IntoFfi};
use bb_common::math::{FPos, Pos, PosError};

pub struct World {
  wid: u32,
}

impl World {
  pub fn new(wid: u32) -> Self { World { wid } }

  pub fn get_block(&self, pos: Pos) -> Result<block::Type, PosError> {
    unsafe {
      let id =
        bb_ffi::bb_world_get_block(self.wid, &bb_ffi::CPos { x: pos.x(), y: pos.y(), z: pos.z() });
      if id == u32::MAX {
        Err(pos.err("invalid position".into()))
      } else {
        // If the kind is invalid here, then `id` must be invalid, so we can panic.
        let kind = block::Kind::from_id(bb_ffi::bb_block_kind_for_type(id)).unwrap();
        Ok(block::Type { kind, state: id })
      }
    }
  }
  pub fn set_block(&self, pos: Pos, ty: block::Type) {
    unsafe {
      bb_ffi::bb_world_set_block(self.wid, &pos.into_ffi(), ty.id());
    }
  }
  pub fn set_block_kind(&self, pos: Pos, kind: block::Kind) {
    unsafe {
      bb_ffi::bb_world_set_block_kind(self.wid, &pos.into_ffi(), kind.id());
    }
  }
  pub fn players(&self) -> impl Iterator<Item = Player> {
    unsafe {
      let players = Box::from_raw(bb_ffi::bb_world_players(self.wid)).into_vec();
      players.into_iter().map(Player::from_ffi)
    }
  }
  /// Spawns a particle in the world. Everyone in render distance will be able
  /// to see this particle.
  pub fn spawn_particle(&self, particle: Particle) {
    unsafe {
      let cparticle = particle.into_ffi();
      bb_ffi::bb_world_spawn_particle(self.wid, &cparticle);
    }
  }
  pub fn raycast(&self, from: FPos, to: FPos, water: bool) -> Option<FPos> {
    unsafe {
      let ptr = bb_ffi::bb_world_raycast(
        &bb_ffi::CFPos { x: from.x(), y: from.y(), z: from.z() },
        &bb_ffi::CFPos { x: to.x(), y: to.y(), z: to.z() },
        bb_ffi::CBool::new(water),
      );
      if ptr.is_null() {
        None
      } else {
        let cpos = Box::from_raw(ptr);
        Some(FPos { x: cpos.x, y: cpos.y, z: cpos.z })
      }
    }
  }
}
