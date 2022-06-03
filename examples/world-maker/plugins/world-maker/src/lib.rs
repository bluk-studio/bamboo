#[macro_use]
extern crate bb_plugin;

use bb_plugin::{
  block,
  command::Command,
  math::FPos,
  particle,
  particle::{Color, Particle},
  PlayerStore,
};
use std::any::Any;

#[derive(Debug)]
struct PlayerInfo {
  brush_size: f64,
}

impl PlayerStore for PlayerInfo {
  fn as_any(&mut self) -> &mut dyn Any { self }
  fn new() -> Self { PlayerInfo { brush_size: 1.0 } }
}

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_block_place(on_place);
  bb_plugin::set_on_tick(on_tick);
  let cmd = Command::new("brush");
  bb_plugin::add_command(&cmd);
}

use bb_plugin::{math::Pos, player::Player};

fn on_place(player: Player, pos: Pos) -> bool {
  player.send_particle(Particle {
    ty:            particle::Type::BlockMarker(block::Kind::Stone.data().default_type()),
    pos:           FPos::new(pos.x as f64 + 0.5, pos.y as f64 + 1.5, pos.z as f64 + 0.5),
    offset:        FPos::new(0.0, 0.0, 0.0),
    count:         1,
    data:          0.0,
    long_distance: false,
  });
  let instance = bb_plugin::instance();
  let mut store = instance.store();
  let info: &mut PlayerInfo = store.player(player.id());
  info.brush_size += 0.5;
  true
}

fn on_tick() {
  let world = bb_plugin::world::World::new(0);
  for player in world.players() {
    let instance = bb_plugin::instance();
    let mut store = instance.store();
    let info: &mut PlayerInfo = store.player(player.id());
    let pos = player.pos();
    let look = player.look_as_vec();
    let from = pos + FPos::new(0.0, 1.5, 0.0);
    let to = from + look * 50.0;

    /*
    if let Some(pos) = world.raycast(from, to, true) {
      let given_x = 1.0;
      let given_y = 1.0;
      let z = -(look.x * given_x + look.y * given_y) / look.z;
      let vec_in_plane = FPos::new(given_x, given_y, z);
      let unit = vec_in_plane / vec_in_plane.size();
      let other_unit = unit.cross(look);

      player.send_particle(Particle {
        ty: particle::Type::Dust(Color { r: 255, g: 255, b: 255 }, 0.5),
        pos,
        offset: FPos::new(0.0, 0.0, 0.0),
        count: 1,
        data: 0.0,
        long_distance: false,
      });
      for angle in 0..30 {
        let angle = angle as f64 / 30.0 * 2.0 * std::f64::consts::PI;
        player.send_particle(Particle {
          ty:            particle::Type::Dust(Color { r: 255, g: 255, b: 255 }, 0.5),
          pos:           pos + unit * angle.cos() + other_unit * angle.sin(),
          offset:        FPos::new(0.0, 0.0, 0.0),
          count:         1,
          data:          0.0,
          long_distance: false,
        });
      }
    }
    */

    if let Some(pos) = world.raycast(from, to, true) {
      player.send_particle(Particle {
        ty: particle::Type::Dust(Color { r: 255, g: 255, b: 255 }, 0.5),
        pos,
        offset: FPos::new(0.0, 0.0, 0.0),
        count: 1,
        data: 0.0,
        long_distance: false,
      });
      if pos.dist_squared(from) < 1.0 {
        return;
      }

      let given_x = 1.0;
      let given_y = 1.0;
      let z = -(look.x * given_x + look.y * given_y) / look.z;
      let vec_in_plane = FPos::new(given_x, given_y, z);
      let unit = vec_in_plane / vec_in_plane.size();
      let other_unit = unit.cross(look);

      for angle in 0..30 {
        let angle = angle as f64 / 30.0 * 2.0 * std::f64::consts::PI;
        // Constant brush size
        let r = info.brush_size * 50.0 / pos.dist(from);
        let to = from + unit * angle.cos() * r + other_unit * angle.sin() * r + look * 50.0;
        /*
        // Brush size changes with distance
        const R: f64 = 10.0;
        let to = from + unit * angle.cos() * R + other_unit * angle.sin() * R + look * 50.0;
        */
        /*
        // Same as constant brush size, but the origin of each raycast is wrong
        const R: f64 = 2.0;
        let from = from + unit * angle.cos() * R + other_unit * angle.sin() * R;
        let to = from + look * 50.0;
        */
        if let Some(pos) = world.raycast(from, to, true) {
          player.send_particle(Particle {
            ty: particle::Type::Dust(Color { r: 255, g: 255, b: 255 }, 0.5),
            pos,
            offset: FPos::new(0.0, 0.0, 0.0),
            count: 1,
            data: 0.0,
            long_distance: false,
          });
        }
      }
    }
  }
}
