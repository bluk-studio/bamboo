mod ty;
mod version;

pub use ty::{Data, Type};
pub use version::TypeConverter;

use crate::{math::AABB, world::World};
use bb_common::{
  math::{FPos, Vec3},
  metadata::Metadata,
};
use parking_lot::{Mutex, MutexGuard, RwLock};
use std::sync::Arc;

pub mod behavior;

use behavior::Behavior;

#[derive(Debug, Clone, Copy)]
pub struct EntityPos {
  pub aabb:     AABB,
  pub vel:      Vec3,
  /// This is determined by the server, so it will always be accurate, unlike
  /// the player grounded field, which is sent to use from the client.
  pub grounded: bool,

  pub yaw:   f32,
  pub pitch: f32,
}

impl EntityPos {
  pub fn new(pos: FPos, size: Vec3) -> Self {
    EntityPos {
      aabb:     AABB::new(pos, size),
      vel:      Vec3::new(0.0, 0.0, 0.0),
      grounded: false,
      yaw:      0.0,
      pitch:    0.0,
    }
  }
}

pub struct Entity {
  /// The unique id for this entity. This is the key used to store entities in
  /// the World.
  eid:      i32,
  /// The position of this entity. Must be valid for all entities.
  pos:      Mutex<EntityPos>,
  /// The type of this entity.
  ty:       Type,
  /// For some entities, such as projectiles, this field is ignored. To make the
  /// entity not disappear when it hits 0 health, overwrite the
  /// `should_despawn` function in `EntityData`.
  health:   Mutex<f32>,
  /// The world this entity is in. Used whenever something changes, and nearby
  /// players need to be notified. This can change if the entity is teleported.
  world:    RwLock<Arc<World>>,
  behavior: Mutex<Box<dyn Behavior + Send>>,

  /// Entity metadata
  meta: Mutex<Metadata>,
}

impl Entity {
  /// Creates a new entity, with default functionality. They will take normal
  /// damage, and despawn if their health hits 0. If you want custom
  /// functionality of any kind, call [`new_custom`].
  pub fn new(eid: i32, ty: Type, world: Arc<World>, pos: FPos, meta: Metadata) -> Self {
    let behavior = behavior::for_entity(ty);
    Entity {
      eid,
      pos: Mutex::new(EntityPos::new(pos, world.entity_converter().get_data(ty).size())),
      ty,
      health: Mutex::new(behavior.max_health()),
      world: RwLock::new(world),
      behavior: Mutex::new(behavior),
      meta: Mutex::new(meta),
    }
  }

  /// Creates a new entity, with the given functionality. This value will be
  /// store within the entity until it despawns.
  pub fn new_custom<B: Behavior + Send + 'static>(
    eid: i32,
    ty: Type,
    pos: FPos,
    world: Arc<World>,
    behavior: B,
    meta: Metadata,
  ) -> Self {
    Entity {
      eid,
      pos: Mutex::new(EntityPos::new(pos, world.entity_converter().get_data(ty).size())),
      ty,
      health: Mutex::new(behavior.max_health()),
      world: RwLock::new(world),
      behavior: Mutex::new(Box::new(behavior)),
      meta: Mutex::new(meta),
    }
  }

  /// Reads this entity's position. This will always be up to date with the
  /// server's known position of this entity. Some clients may be behind this
  /// position (by up to 1/20 of a second).
  ///
  /// This returns everything about the entity's position, including its
  /// velocity, pitch, yaw, etc. If you just need the position, then
  /// [`fpos`](Self::fpos) will be easier to use.
  pub fn pos(&self) -> EntityPos { *self.pos.lock() }

  /// Returns this entity's position.
  pub fn fpos(&self) -> FPos { self.pos().aabb.pos }

  /// Returns the unique id for this entity.
  pub fn eid(&self) -> i32 { self.eid }

  /// Returns this entity's type. This can be used to send spawn packets to
  /// clients.
  pub fn ty(&self) -> Type { self.ty }

  /// Returns this entity's health.
  pub fn health(&self) -> f32 { *self.health.lock() }

  /// Returns true if this entity should despawn.
  pub fn should_despawn(&self) -> bool { self.behavior.lock().should_despawn(self.health()).0 }

  /// Returns the amount of exp stored in this entity. This is just the amount
  /// for an exp orb, but it is also used to find out how much exp an entity
  /// will drop when killed.
  pub fn exp_count(&self) -> i32 { self.behavior.lock().exp_count() }

  /// Sets this entity's velocity. This will send velocity updates to nearby
  /// players, and will affect how the entity moves on the next tick.
  pub fn set_vel(&self, vel: Vec3) {
    self.pos.lock().vel = vel;
    self.world.read().send_entity_vel(self.fpos().chunk(), self.eid, vel);
  }

  /// Called 20 times a second. Calling this more/less frequently will break
  /// things.
  pub(crate) fn tick(&self) -> bool {
    // We don't actually have a race condition here, unless tick() is called at the
    // same time from multiple places (which would be a Bad Thing). Because we can't
    // modify `self.pos` from anywhere else (simply because the functions don't
    // exist), then we won't overwrite changed data by unlocking and re-locking this
    // mutex.
    let mut p = self.pos.lock().clone();
    let old = p.aabb;
    let old_vel = p.vel;
    if self.behavior.lock().tick(self, &mut p).0 {
      return true;
    }
    let w = self.world.read();
    if p.aabb.pos != old.pos {
      let nearby = w.nearby_colliders(p.aabb);
      // Make tmp so that old can be used in world.send_entity_pos.
      let mut tmp = old;
      if let Some(res) = tmp.move_towards((p.aabb.pos - old.pos).into(), &nearby) {
        if res.axis.x != 0.0 {
          p.vel.x = 0.0
        } else if res.axis.y != 0.0 {
          p.vel.y = 0.0
        } else if res.axis.z != 0.0 {
          p.vel.z = 0.0
        }
        if res.axis.y > 0.0 {
          p.grounded = true;
        }
        p.aabb = tmp;
      }
      *self.pos.lock() = p;
      self.world.read().send_entity_pos(self.eid, old.pos, p.aabb.pos, false);
    } else {
      *self.pos.lock() = p;
    }
    if p.vel != old_vel {
      self.world.read().send_entity_vel(old.pos.chunk(), self.eid, p.vel);
    }
    false
  }

  /// Returns all of this entity's metadata.
  pub fn metadata(&self) -> MutexGuard<'_, Metadata> { self.meta.lock() }
}
