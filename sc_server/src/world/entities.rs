use super::World;
use crate::{entity, entity::Entity, player::Player};
use parking_lot::RwLockReadGuard;
use sc_common::{
  math::{ChunkPos, FPos, Vec3},
  net::cb,
  util::UUID,
};
use std::{
  collections::HashMap,
  sync::{atomic::Ordering, Arc},
};

impl World {
  pub fn summon(self: &Arc<Self>, ty: entity::Type, pos: FPos) -> i32 {
    let eid = self.eid();
    let ent = Entity::new(eid, ty, self.clone(), pos);

    for p in self.players().iter().in_view(pos.chunk()) {
      self.send_entity_spawn(p, &ent);
    }

    self.add_entity(eid, ent);
    eid
  }

  /// Sends entity velocity packets to everyone in view of `pos`.
  pub(crate) fn send_entity_vel(&self, pos: ChunkPos, eid: i32, vel: Vec3) {
    for p in self.players().iter().in_view(pos) {
      p.send(cb::Packet::EntityVelocity {
        eid,
        x: vel.fixed_x(),
        y: vel.fixed_y(),
        z: vel.fixed_z(),
      });
    }
  }

  /// Sends entity position packets to everyone in view of `old`.
  pub(crate) fn send_entity_pos(&self, eid: i32, old: FPos, new: FPos, on_ground: bool) {
    for p in self.players().iter().in_view(old.chunk()) {
      let x = new.x() - old.x();
      let y = new.y() - old.y();
      let z = new.z() - old.z();
      let x = (x * 4096.0).round() as i16;
      let y = (y * 4096.0).round() as i16;
      let z = (z * 4096.0).round() as i16;
      /*
      let abs_pos;
      if p.ver() == ProtocolVersion::V1_8 {
        dx *= 32.0;
        dy *= 32.0;
        dz *= 32.0;
        if dx.abs() > i8::MAX.into() || dy.abs() > i8::MAX.into() || dz.abs() > i8::MAX.into() {
          abs_pos = true;
        } else {
          // As truncates any negative floats to 0, but just copies the bits for i8 -> u8
          d_x_v1_8 = dx.round() as i8;
          d_y_v1_8 = dy.round() as i8;
          d_z_v1_8 = dz.round() as i8;
          abs_pos = false;
        }
      } else {
        dx *= 4096.0;
        dy *= 4096.0;
        dz *= 4096.0;
        // 32 * 128 * 8 = 16384, which is the max value of an i16. So if we have more
        // than an 8 block delta, we cannot send a relative movement packet.
        if dx.abs() > i16::MAX.into() || dy.abs() > i16::MAX.into() || dz.abs() > i16::MAX.into() {
          abs_pos = true;
        } else {
          d_x_v1_9 = dx.round() as i16;
          d_y_v1_9 = dy.round() as i16;
          d_z_v1_9 = dz.round() as i16;
          abs_pos = false;
        }
      };
      */
      p.send(cb::Packet::EntityMove { eid, x, y, z, on_ground });
    }
  }

  pub fn entities(&self) -> RwLockReadGuard<'_, HashMap<i32, Arc<Entity>>> { self.entities.read() }

  fn add_entity(&self, eid: i32, entity: Entity) {
    self.entities.write().insert(eid, Arc::new(entity));
  }

  #[allow(clippy::if_same_then_else)]
  fn send_entity_spawn(&self, player: &Player, ent: &Entity) {
    info!("spawning entity {:?}", ent.ty());
    let p = ent.pos();
    let id = ent.ty().id();
    let old_id = self.entity_converter().to_old(id, player.ver().block());
    info!("modern id: {}", id);
    info!("old id: {:?}", old_id);
    if ent.ty() == entity::Type::ExperienceOrb {
      // player.send(cb::Packet::SpawnEntityExperienceOrb {
      //   entity_id: ent.eid(),
      //   x_v1_8:    Some(p.aabb.pos.fixed_x()),
      //   x_v1_9:    Some(p.aabb.pos.x()),
      //   y_v1_8:    Some(p.aabb.pos.fixed_y()),
      //   y_v1_9:    Some(p.aabb.pos.y()),
      //   z_v1_8:    Some(p.aabb.pos.fixed_z()),
      //   z_v1_9:    Some(p.aabb.pos.z()),
      //   count:     ent.exp_count() as i16,
      // });
      todo!();
    } else if ent.ty() == entity::Type::Painting {
      // player.send(cb::Packet::SpawnEntityPainting {
      //   entity_id:        ent.eid(),
      //   entity_uuid_v1_9: Some(UUID::from_u128(0)),
      //   title_v1_8:       Some("hello".into()),
      //   title_v1_13:      Some(0),
      //   location:         p.aabb.pos.block(),
      //   direction:        (p.yaw / 360.0 * 8.0 + 4.0) as u8,
      // });
      todo!();
    } else if ent.ty().is_living() {
      player.send(cb::Packet::SpawnLivingEntity {
        eid:      ent.eid(),
        // 1.18 clients will not render mobs that have the same UUID
        id:       UUID::random(),
        ty:       old_id,
        x:        p.aabb.pos.x(),
        y:        p.aabb.pos.y(),
        z:        p.aabb.pos.z(),
        yaw:      (p.yaw / 360.0 * 256.0) as i8,
        pitch:    (p.pitch / 360.0 * 256.0) as i8,
        head_yaw: 0,
        vel_x:    p.vel.fixed_x(),
        vel_y:    p.vel.fixed_y(),
        vel_z:    p.vel.fixed_z(),
      });
    } else {
      // Data is some data specific to that entity. If it is non-zero, then velocity
      // is present.
      let _data: i32 = 0;
      // player.send(cb::Packet::SpawnEntity {
      //   entity_id:        ent.eid(),
      //   object_uuid_v1_9: Some(UUID::from_u128(0)),
      //   type_v1_8:        Some(old_id as i8),
      //   type_v1_14:       Some(old_id as i32),
      //   x_v1_8:           Some(p.aabb.pos.fixed_x()),
      //   x_v1_9:           Some(p.aabb.pos.x()),
      //   y_v1_8:           Some(p.aabb.pos.fixed_y()),
      //   y_v1_9:           Some(p.aabb.pos.y()),
      //   z_v1_8:           Some(p.aabb.pos.fixed_z()),
      //   z_v1_9:           Some(p.aabb.pos.z()),
      //   pitch:            (p.pitch / 360.0 * 256.0) as i8,
      //   yaw:              (p.yaw / 360.0 * 256.0) as i8,
      //   object_data_v1_8: Some(data.to_le_bytes().to_vec()),
      //   object_data_v1_9: Some(data),
      //   velocity_x_v1_9:  Some(p.vel.fixed_x()),
      //   velocity_y_v1_9:  Some(p.vel.fixed_y()),
      //   velocity_z_v1_9:  Some(p.vel.fixed_z()),
      // });
      todo!();
    }
  }
}
