// use super::wrapper::*;
// use crate::{block, player::Player};
// use common::math::Pos;
// use rutie::{AnyObject, Fixnum, Module, Object, VM};
// use std::sync::{mpsc, Arc};
use super::{
  types::{SlBlockKind, SlPlayer, SlPos},
  PluginManager, Sugarcane,
};
use crate::{block, player::Player, world::WorldManager};
use sc_common::math::Pos;
use std::{fs, path::Path, sync::Arc};
use sugarlang::{
  path,
  runtime::{LockedEnv, Path as TyPath, Var},
  SlError, Sugarlang,
};

/// A wrapper struct for a Ruby plugin. This is used to execute Ruby code
/// whenever an event happens.
pub struct Plugin {
  name: String,
  sl:   Option<Sugarlang>,
  sc:   Sugarcane,
}

impl Plugin {
  //   /// Creates a new plugin. The name should be the name of the module (for
  //   /// debugging) and the Module should be the ruby module for this plugin.
  pub fn new(idx: usize, name: String, wm: Arc<WorldManager>) -> Self {
    Plugin { sc: Sugarcane::new(idx, name.clone(), wm), name, sl: None }
  }

  /// This replaces the plugins envrionment with a new one, and then parses the
  /// given file as a sugarlang source file.
  pub fn load_from_file(&mut self, path: &Path, manager: &PluginManager) {
    self.sl = None;
    let mut sl = Sugarlang::new();
    sl.set_color(manager.use_color());
    PluginManager::add_builtins(&mut sl);
    match fs::read_to_string(path) {
      Ok(src) => match sl.parse_file(&path!(main), path, src) {
        Ok(_) => {
          self.sl = Some(sl);
        }
        Err(err) => {
          self.sl = Some(sl);
          self.print_err(err);
          self.sl = None;
        }
      },
      Err(err) => {
        warn!("{}", err);
      }
    }
  }

  pub fn lock_env(&mut self) -> LockedEnv {
    let sl = self.sl.as_mut().unwrap();
    let (env, files) = sl.env_files();
    env.lock(files)
  }
  /// Returns a cloned Sugarcane struct. This should be used to call Sugarlang
  /// functions.
  pub fn sc(&self) -> Sugarcane {
    self.sc.clone()
  }

  pub fn call_init(&self) {
    self.call(path!(main), "init", vec![self.sc.clone().into()]);
  }
  pub fn call_on_block_place(&self, player: Arc<Player>, pos: Pos, kind: block::Kind) {
    self.call(
      path!(main),
      "on_block_place",
      vec![
        self.sc.clone().into(),
        SlPlayer::from(player).into(),
        SlPos::from(pos).into(),
        SlBlockKind::from(kind).into(),
      ],
    );
  }

  pub fn call(&self, path: TyPath, name: &str, args: Vec<Var>) {
    match &self.sl {
      Some(sl) => {
        match sl.call_args(path, name, args.into_iter().map(|v| v.into_ref()).collect()) {
          Ok(_) => {}
          Err(e) => self.print_err(e),
        }
      }
      None => {}
    }
  }

  pub fn print_err<E: SlError>(&self, err: E) {
    match &self.sl {
      Some(sl) => warn!("error in plugin {}:\n{}", self.name, sl.gen_err(err)),
      None => {}
    }
  }

  /// This replaces the plugin envrionment with a new one, and then parses all
  /// of the files ending in `.sug` in the given directory.
  pub fn load_from_dir(_path: &Path) {}
}