use super::{Behavior, Entity, EntityPos, ShouldDespawn};

pub struct SnowballBehavior {
  _effect: Option<i32>,
}

impl Default for SnowballBehavior {
  fn default() -> Self { SnowballBehavior { _effect: None } }
}

impl Behavior for SnowballBehavior {
  fn tick(&mut self, _ent: &Entity, p: &mut EntityPos) -> ShouldDespawn {
    let vel = p.vel;
    p.aabb.pos += vel;
    // This is for all projectiles. It is totally different on living entities.
    p.vel.x *= 0.99;
    p.vel.y *= 0.99;
    p.vel.z *= 0.99;
    if !p.grounded {
      p.vel.y -= 0.03;
    }
    ShouldDespawn(false)
  }
}