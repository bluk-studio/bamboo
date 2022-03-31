use super::{Bamboo, PandaPlugin};
use bb_common::util::{chat::Color, Chat};
use panda::{
  define_ty,
  docs::{markdown, MarkdownSection},
  parse::token::Span,
  path,
  runtime::{RuntimeError, Var},
  Panda,
};

pub mod block;
pub mod chat;
pub mod command;
pub mod item;
pub mod player;
pub mod util;
pub mod world;

use command::PdCommand;
use world::{gen::PdBiome, PdWorld};

macro_rules! add_from {
  ( $ty:ty, $new_ty:ident ) => {
    impl From<$ty> for $new_ty {
      fn from(inner: $ty) -> $new_ty { $new_ty { inner } }
    }
  };
}

macro_rules! wrap {
  ( $ty:ty, $new_ty:ident ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      pub(super) inner: $ty,
    }

    add_from!($ty, $new_ty);
  };

  ( $ty:ty, $new_ty:ident, $($extra:ident: $extra_ty:ty),* ) => {
    #[derive(Clone, Debug)]
    pub struct $new_ty {
      pub(super) inner:  $ty,
      $(
        pub(super) $extra: $extra_ty,
      )*
    }
  };
}

// Only want these to be public to local files.
use add_from;
use wrap;

/// This is a handle into the Bamboo server. It allows you to modify the
/// world, add commands, lookup players, and more. It will be passed to every
/// callback, so you should not store this in a global (although you can if you
/// need to).
#[define_ty(path = "bamboo::Bamboo")]
impl Bamboo {
  /// Adds a command to the server.
  ///
  /// # Example
  ///
  /// ```
  /// fn main() {
  ///   c = Command::new("setblock", handle_setblock)
  ///   bamboo::instance().add_command(c)
  /// }
  ///
  /// fn handle_setblock(player, args) {
  ///   bamboo::info("ran setblock!")
  /// }
  /// ```
  pub fn add_command(&self, command: &PdCommand) -> Result<(), RuntimeError> {
    let wm = self.wm.clone();
    let wm2 = self.wm.clone();
    let cb = match command.callback.clone() {
      Some(cb) => cb,
      None => {
        return Err(RuntimeError::custom(
          "cannot pass in child command! you must pass in a command created from `Command::new`",
          Span::call_site(),
        ))
      }
    };
    let command = command.inner.lock().unwrap().clone();
    let idx = self.idx;
    wm.commands().add(command, move |_, player, args| {
      let wm = wm2.clone();
      let mut cb = cb.clone();
      {
        // We need this awkward scoping setup to avoid borrowing errors, and to make
        // sure `lock` doesn't get sent between threads.
        let mut err = None;
        let mut has_err = false;
        {
          let mut lock = wm.plugins().plugins.lock();
          let plugin = &mut lock[idx];
          let panda = plugin.unwrap_panda();
          if let Err(e) = cb.call(
            &mut panda.lock_env(),
            vec![
              player.map(|p| player::PdPlayer::from(p.clone()).into()).unwrap_or(Var::None),
              args.iter().map(|arg| command::sl_from_arg(arg.clone())).collect::<Vec<Var>>().into(),
            ],
          ) {
            err = Some(e);
            has_err = true;
          }
          if let Some(e) = err {
            panda.print_err(e);
          }
        }
        if has_err {
          if let Some(p) = player {
            let mut out = Chat::new("");
            out.add("Error executing command: ").color(Color::Red);
            out.add(format!("`{}` encountered an internal error", args[0].lit()));
            p.send_message(&out);
          }
        }
      }
    });
    Ok(())
  }
  /// Adds a new biome to the server. This works ontop of a terrain generator.
  /// By default, each of the biomes are chosen at random, for various regions
  /// of the map. Then, the biome generation takes place in each of these
  /// regions. The behavior of an individual biome can be overriden using this
  /// function.
  ///
  /// # Example
  ///
  /// ```
  /// fn main() {
  ///   biome = Biome::new("desert")
  ///
  ///   bb = bamboo::instance()
  ///   bb.add_biome(biome)
  /// }
  /// ```
  ///
  /// See the `Biome` docs for more.
  pub fn add_biome(&self, _biome: &PdBiome) -> Result<(), RuntimeError> { Ok(()) }

  /// Locks the internal data. If the internal data is already locked, this will
  /// continue trying to lock that data.
  pub fn lock(&self) -> Var {
    loop {
      match self.data.lock().take() {
        Some(v) => return v.into(),
        None => std::thread::yield_now(),
      }
    }
  }

  /// Locks the internal data, stores the given value, and then unlocks it. Note
  /// that this should be avoided if you have multiple threads modifying
  /// `data`. This can cause a race condition if you are not careful. If you
  /// need to hold onto a lock, then call `Bamboo::lock` instead.
  pub fn store(&self, data: Var) -> Result<(), RuntimeError> {
    let mut lock = self.data.lock();
    if lock.is_none() {
      return Err(RuntimeError::custom("data is already locked!", Span::call_site()));
    }
    lock.replace(data.into());
    Ok(())
  }

  /// Releases the lock on the internal data. This will move the given data back
  /// into the lock. Note that if the lock is not held, this will return an
  /// error.
  pub fn unlock(&self, data: Var) -> Result<(), RuntimeError> {
    let prev = self.data.lock().replace(data.into());
    if let Some(prev) = prev {
      Err(RuntimeError::custom(
        format!("was not locked! existing data: {prev:?}"),
        Span::call_site(),
      ))
    } else {
      Ok(())
    }
  }

  /// Broadcasts the given chat message.
  pub fn broadcast(&self, chat: &chat::PdChat) {
    self.wm.broadcast(chat.inner.lock().unwrap().clone());
  }

  /// Returns the default world.
  pub fn default_world(&self) -> PdWorld { self.wm.default_world().into() }
}

fn format(args: &[Var]) -> String {
  let mut msg = String::new();
  let mut iter = args.iter();
  if let Some(a) = iter.next() {
    msg += &format!("{}", a);
  }
  for a in iter {
    msg += &format!(" {}", a);
  }
  msg
}

impl PandaPlugin {
  pub fn add_builtins(&self, sl: &mut Panda) {
    let bb = self.bb();
    sl.add_builtin_fn(path!(bamboo::instance), false, move |_env, _slf, args, pos| {
      RuntimeError::check_arg_len(&args, 0, pos)?;
      Ok(bb.clone().into())
    });
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(bamboo::trace), false, move |_env, _slf, args, _pos| {
        trace!("`{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(bamboo::debug), false, move |_env, _slf, args, _pos| {
        debug!("`{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(bamboo::info), false, move |_env, _slf, args, _pos| {
        info!("`{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(bamboo::warn), false, move |_env, _slf, args, _pos| {
        warn!("`{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    {
      let name = self.name().clone();
      sl.add_builtin_fn(path!(bamboo::error), false, move |_env, _slf, args, _pos| {
        error!("`{}`: {}", name, format(&args));
        Ok(Var::None)
      });
    }
    sl.add_builtin_ty::<Bamboo>();
    sl.add_builtin_ty::<util::PdPos>();
    sl.add_builtin_ty::<util::PdFPos>();
    sl.add_builtin_ty::<block::PdBlockKind>();
    sl.add_builtin_ty::<chat::PdChat>();
    sl.add_builtin_ty::<chat::PdChatSection>();
    sl.add_builtin_ty::<item::PdClickWindow>();
    sl.add_builtin_ty::<item::PdInventory>();
    sl.add_builtin_ty::<item::PdStack>();
    sl.add_builtin_ty::<item::PdUI>();
    sl.add_builtin_ty::<command::PdCommand>();
    sl.add_builtin_ty::<player::PdPlayer>();
    sl.add_builtin_ty::<world::PdWorld>();
    sl.add_builtin_ty::<world::gen::PdBiome>();
  }

  pub fn generate_docs(&self, sl: &Panda) {
    let docs = sl.generate_docs(
      &[
        (
          path!(bamboo),
          markdown!(
            /// The Bamboo API. This is how all panda code can interact
            /// with the Bamboo minecraft server. To get started with writing
            /// a plugin, create a directory called `plugins` next to the server.
            /// Inside that directory, create a file named something like `hello.sug`.
            /// In that file, put the following code:
            ///
            /// ```
            /// fn init() {
            ///   bamboo::info("Hello world")
            /// }
            /// ```
            ///
            /// To start doing more things with your plugin, check out the docs
            /// for the `Bamboo` type. You can access an instance of it like so:
            ///
            /// ```
            /// fn init() {
            ///   bb = bamboo::instance()
            ///   bb.broadcast("This will show up in chat for everyone!")
            /// }
            /// ```
          ),
        ),
        (
          path!(bamboo::block),
          markdown!(
            /// This module handles blocks. It includes types to manage block kinds,
            /// block types, and any other data for blocks.
          ),
        ),
        (
          path!(bamboo::chat),
          markdown!(
            /// This module handles chat messages. It includes a chat message (`Chat`),
            /// and a chat section type (`ChatSection`).
          ),
        ),
        (
          path!(bamboo::command),
          markdown!(
            /// This module handles commands. Since minecraft 1.13, commands have gotten
            /// somewhat complex. This module allows you to build a command from a tree
            /// of arguments, and parse those commands from the client.
            ///
            /// For 1.8 to 1.12 clients, auto complete is implemented on the server, so
            /// it seems similar to 1.13+. You should not need to worry about this. You
            /// should be focused on building a command that takes the right arguments,
            /// which will then be handled by the server and client for you.
            ///
            /// Here is an example of creating a simple command:
            ///
            /// ```
            /// fn init() {
            ///   // The first argument is the name of the command, and the second argument
            ///   // is the function to call when the command is executed by a client.
            ///   c = Command::new("fill", handle_fill)
            ///   // This will expect the word `rect` to be inserted into the command string.
            ///   c.add_lit("rect")
            ///     .add_arg_block_pos("min")
            ///     .add_arg_block_pos("max")
            ///   // This will expect the word `circle` to be inserted into the command string.
            ///   // Since we are working with the original `c` struct, this is added as a
            ///   // second path in the tree, next to the `rect` path.
            ///   c.add_lit("rect")
            ///     .add_arg_block_pos("center")
            ///     .add_arg_float("radius")
            /// }
            ///
            /// // `bb` is the Bamboo struct, which gives you access to everything on
            /// // the server.
            /// // `player` is the player who sent this command.
            /// // `args` is the parsed arguments to your command. Each item in this array
            /// // is typed according to the arguments. So, the first item will be a string
            /// // with the text "fill", the second will be a string will the text "rect"
            /// // or "circle", the third item will be a block position, and the last item will
            /// // be another block position or a float.
            ///
            /// fn handle_fill(bb, player, args) {
            ///   player.send_message("You just ran /fill!")
            /// }
            /// ```
            ///
            /// This will build a command with a tree like so:
            ///
            /// ```
            ///      fill
            ///    /      \
            /// "rect"  "circle"
            ///   |        |
            /// `pos`    `pos`
            ///   |        |
            /// `pos`   `float`
            /// ```
            ///
            /// This will parse all of these commands:
            ///
            /// ```
            /// /fill rect -3 -2 -1 4 5 6
            /// /fill circle ~ ~ ~ 3.5
            /// /fill rect ~-10 ~-10 ~-10 ~10 ~10 ~10
            /// ```
          ),
        ),
        (
          path!(bamboo::player),
          markdown!(
            /// This module handles the player. It only includes one struct, called `Player`.
            /// Players are very complex, so it makes sense not to bundle the player in with
            /// entities.
          ),
        ),
        (
          path!(bamboo::util),
          markdown!(
            /// This module includes various utilities, such as block positions, chunk positions,
            /// byte buffers, things like UUID, and more.
          ),
        ),
        (
          path!(bamboo::world),
          markdown!(
            /// This module includes a World and WorldManager. The World allows you to spawn
            /// in entities, change blocks, mess with players, etc. The WorldManager allows
            /// you to teleport people between worlds, create new worlds, etc.
          ),
        ),
        (
          path!(bamboo::world::gen),
          markdown!(
            /// This modules is for everything related to terrain generation. This is a complex
            /// module, simply becuase of how much there is to do.
          ),
        ),
      ],
      &[
        (
          path!(bamboo::trace),
          markdown!(
            /// Prints out the given arguments as a `trace` log message.
          ),
        ),
        (
          path!(bamboo::debug),
          markdown!(
            /// Prints out the given arguments as a `debug` log message.
          ),
        ),
        (
          path!(bamboo::info),
          markdown!(
            /// Prints out the given arguments as an `info` log message.
            ///
            /// # Example
            ///
            /// ```
            /// bamboo::info("some information")
            /// bamboo::info(5, 6)
            /// bamboo::info(my_vars, other, info)
            /// ```
          ),
        ),
        (
          path!(bamboo::warn),
          markdown!(
            /// Prints out the given arguments as a `warn` log message.
          ),
        ),
        (
          path!(bamboo::error),
          markdown!(
            /// Prints out the given arguments as an `error` log message.
          ),
        ),
      ],
    );
    docs.save("target/sl_docs");
  }
}