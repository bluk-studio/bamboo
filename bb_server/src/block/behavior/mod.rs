use super::{Block, Data, Kind, Type};
use crate::{item::Stack, player::Player, world::World};
use bb_common::{math::Pos, util::Face};
use std::{collections::HashMap, sync::Arc};

mod impls;

pub trait Behavior: Send + Sync {
  /// Called when a block is about to be placed.
  ///
  /// This should handle things like logs rotating or torches not placing on
  /// ceilings.
  fn place(&self, data: &Data, pos: Pos, face: Face) -> Type {
    let _ = (pos, face);
    data.default_type()
  }
  /// Called after this block is placed. The `block` is the block that was
  /// placed.
  ///
  /// This should handle falling blocks spawning after the block is placed.
  fn update_place(&self, world: &Arc<World>, block: Block) { let _ = (world, block); }
  /// Called whenever a block is updated next to `block`. `old` and `new` will
  /// both have the same position, and will be next to `block`.
  ///
  /// This should handle falling blocks being created after a block is broken
  /// underneath it.
  fn update(&self, world: &Arc<World>, block: Block, old: Block, new: Block) {
    let _ = (world, block, old, new);
  }
  /// Called when the block is placed. If the block needs to store extra
  /// information, a [`TileEntity`] should be returned.
  ///
  /// Blocks such as chests, juke boxes, and furnaces should return a tile
  /// entity here.
  fn create_tile_entity(&self) -> Option<Box<dyn TileEntity>> { None }

  /// Called when a player right clicks on this block. If this returns `true`,
  /// the event was handled, and a block should not be placed.
  fn interact(&self, block: Block, player: &Arc<Player>) -> bool {
    let _ = (block, player);
    false
  }
  /// Returns the drops for the given block. The default drops for this block
  /// are collected from the vanilla client, but this may require some
  /// overrides. Returning [`BlockDrops::Normal`] will use the vanilla drops,
  /// and returning [`BlockDrops::Custom`] will override the vanilla drops
  /// with the given [`Drops`].
  fn drops(&self, block: Block) -> BlockDrops {
    let _ = block;
    BlockDrops::Normal
  }
}

// TODO: This needs to be able to store it's data to disk.
pub trait TileEntity: Send {}

pub fn make_behaviors() -> HashMap<Kind, Box<dyn Behavior>> {
  let mut out: HashMap<_, Box<dyn Behavior>> = HashMap::new();
  /*
  macro_rules! b {
    ( $kind:ident $(| $kind2:ident)* => $impl:expr ) => {
      out.insert(Kind::$kind, Box::new($impl));
      $(
        out.insert(Kind::$kind2, Box::new($impl));
      )*
    };
    ( *color*$kind:ident => $impl:expr ) => {
      let name = concat_idents!(Red, $kind);
      out.insert(Kind::name, Box::new($impl));
    }
  }
  */
  bb_plugin_macros::behavior! {
    *wood* = Oak, Birch, Spruce, DarkOak, Acacia, Jungle;
    *color* = White, Orange, Magenta, LightBlue, Yellow, Lime, Pink, Gray, LightGray, Cyan, Purple, Blue, Brown, Green, Red, Black;

    *wood*Log => impls::Log;
    Stripped*wood*Log => impls::Log;

    Sand | RedSand | Gravel => impls::Falling;

    CraftingTable => impls::CraftingTable;

    *color*Bed => impls::Bed;
  };
  /*
  b!(OakLog => impls::Log);
  b!(BirchLog => impls::Log);
  b!(SpruceLog => impls::Log);
  b!(DarkOakLog => impls::Log);
  b!(AcaciaLog => impls::Log);
  b!(JungleLog => impls::Log);
  b!(StrippedOakLog => impls::Log);
  b!(StrippedBirchLog => impls::Log);
  b!(StrippedSpruceLog => impls::Log);
  b!(StrippedDarkOakLog => impls::Log);
  b!(StrippedAcaciaLog => impls::Log);
  b!(StrippedJungleLog => impls::Log);

  b!(Sand | RedSand | Gravel => impls::Falling);

  b!(CraftingTable => impls::CraftingTable);

  b!(*color*Bed => impls::Bed);
  */
  out
}

/// A collection of things to drop from a block or entity.
#[derive(Debug, Clone, Default)]
pub struct Drops {
  pub exp:   i32,
  pub items: Vec<Stack>,
}

pub enum BlockDrops {
  Normal,
  Custom(Drops),
}

impl Drops {
  pub fn empty() -> Self { Drops::default() }
}
