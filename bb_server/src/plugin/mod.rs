mod json;
pub mod panda;
mod plugin;
pub mod socket;

pub use json::*;
pub use plugin::{
  Plugin, PluginEvent, PluginImpl, PluginMessage, PluginRequest, ServerEvent, ServerMessage,
  ServerReply,
};

use panda::PandaPlugin;
use socket::SocketManager;

use crate::{block, player::Player, world::WorldManager};
use bb_common::{config::Config, math::Pos, net::sb::ClickWindow, util::Chat};
use parking_lot::Mutex;
use std::{fmt, fs, sync::Arc};
use sugarlang::runtime::VarSend;

#[derive(Debug)]
pub enum Event {
  Init,
  OnBlockPlace(Arc<Player>, Pos, block::Kind),
}

/// A struct that manages all plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
pub struct PluginManager {
  plugins: Mutex<Vec<Plugin>>,
}

#[derive(Clone)]
pub struct Bamboo {
  // Index into plugins array
  idx:    usize,
  plugin: String,
  wm:     Arc<WorldManager>,
  // Locking this removes the value. If the value is none, then this enters a wait loop until there
  // is a value present.
  //
  // This is not by any means "fast", but it will work as long as a thread doesn't lock this for
  // too long.
  data:   Arc<Mutex<Option<VarSend>>>,
}

impl Bamboo {
  pub fn new(idx: usize, plugin: String, wm: Arc<WorldManager>) -> Self {
    Bamboo { idx, plugin, wm, data: Arc::new(Mutex::new(Some(VarSend::None))) }
  }
}

impl fmt::Debug for Bamboo {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Bamboo {{}}") }
}

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { PluginManager { plugins: Mutex::new(vec![]) } }

  /// Returns true if plugins should print error messages with colors.
  pub fn use_color(&self) -> bool { true }

  pub fn run(&self, wm: Arc<WorldManager>) {
    // let mut ctx = Context::new();
    // ctx.register_global_class::<Bamboo>().unwrap();
    // let _o = ctx.construct_object();
    // ctx.register_global_property(
    //   "sc",
    //   Object::native_object(Box::new(Bamboo::new(wm.clone()))),
    //   Attribute::all(),
    // );
    // self.load(&mut ctx, wm);
    //
    // let rx = self.rx.lock().unwrap();
    // self.handle_event(Event::Init);
    // loop {
    //   if let Ok(e) = rx.recv() {
    //     self.handle_event(e);
    //   }
    // }
  }

  /// Loads all plugins from disk. Call this to reload all plugins.
  pub fn load(&self, wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock();
    plugins.clear();

    let mut sockets = SocketManager::new(wm.clone());

    for f in fs::read_dir("plugins").unwrap() {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_file() {
        /*
        let path = f.path();
        info!("found plugin at {}", path.to_str().unwrap());
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let mut p = Plugin::new(plugins.len(), name, wm.clone());

        p.load_from_file(&path, self);
        p.call_init();
        plugins.push(p);
        */
      } else if m.is_dir() {
        let path = f.path();
        let config = Config::new(
          path.join("plugin.yml").to_str().unwrap(),
          path.join("plugin-default.yml").to_str().unwrap(),
          include_str!("plugin.yml"),
        );
        if !config.get::<_, bool>("enabled") {
          continue;
        }
        let ty: String = config.get("type");
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        if ty == "socket" {
          info!("found socket plugin at {}", path.to_str().unwrap());
          if let Some(plugin) = sockets.add(name.clone(), f.path()) {
            plugins.push(Plugin::new(config, name, plugin));
          }
        } else if ty == "panda" {
          let main_path = f.path().join("main.sug");
          if main_path.exists() && main_path.is_file() {
            info!("found panda plugin at {}", main_path.to_str().unwrap());
            let name = f.path().file_stem().unwrap().to_str().unwrap().to_string();
            let mut p = PandaPlugin::new(plugins.len(), name.clone(), wm.clone());

            p.load_from_dir(&f.path(), self);
            p.call_init();
            plugins.push(Plugin::new(config, name, p));
          } else {
            error!("plugin `{name}` does not have a `main.sug` file");
          }
        } else {
          error!("plugin `{name}` has invalid plugin type: `{ty}`");
        }
      }
    }

    let plugins = sockets.take_plugins();
    std::thread::spawn(|| {
      sockets.listen();
    });
    for plug in plugins {
      plug.wait_for_ready().unwrap();
      plug.clone().spawn_listener();
    }
  }

  fn message(&self, msg: ServerMessage) {
    self.plugins.lock().retain(|p| p.call(msg.clone()).is_ok());
  }
  fn event(&self, player: Arc<Player>, event: ServerEvent) {
    self.message(ServerMessage::Event { player, event });
  }
  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, block: block::Type) {
    self.event(player, ServerEvent::BlockPlace { pos, block });
  }
  pub fn on_chat_message(&self, player: Arc<Player>, message: Chat) {
    self.event(player, ServerEvent::Chat { text: message.to_plain() });
  }
  pub fn on_player_join(&self, player: Arc<Player>) {
    self.event(player, ServerEvent::PlayerJoin {});
  }
  pub fn on_click_window(&self, player: Arc<Player>, slot: i32, mode: ClickWindow) -> bool {
    let mut allow = true;
    for p in self.plugins.lock().iter() {
      /*
      if !p.call(player.clone(), slot, mode.clone()) {
        allow = false
      }
      */
    }
    allow
  }
}
