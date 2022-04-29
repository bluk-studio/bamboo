use super::{Block, Data, Kind, Type};
use crate::world::World;
use bb_common::{math::Pos, util::Face};
use std::{collections::HashMap, sync::Arc};

mod impls;

pub trait Behavior: Send + Sync {
  fn place(&self, data: &Data, pos: Pos, face: Face) -> Type {
    let _ = (pos, face);
    data.default_type()
  }
  fn update(&self, world: &Arc<World>, block: Block, old: Block, new: Block) {
    let _ = (world, block, old, new);
  }
  fn create_tile_entity(&self) -> Option<Box<dyn TileEntity>> { None }
}

// TODO: This needs to be able to store it's data to disk.
pub trait TileEntity: Send {}

pub fn make_behaviors() -> HashMap<Kind, Box<dyn Behavior>> {
  let mut out: HashMap<_, Box<dyn Behavior>> = HashMap::new();
  macro_rules! behaviors {
    ( $($kind:ident $(| $kind2:ident)* => $impl:expr,)* ) => {
      $(
        out.insert(Kind::$kind, Box::new($impl));
        $(
          out.insert(Kind::$kind2, Box::new($impl));
        )*
      )*
    }
  }
  behaviors! {
    OakLog => impls::Log,
    BirchLog => impls::Log,
    SpruceLog => impls::Log,
    DarkOakLog => impls::Log,
    AcaciaLog => impls::Log,
    JungleLog => impls::Log,
    StrippedOakLog => impls::Log,
    StrippedBirchLog => impls::Log,
    StrippedSpruceLog => impls::Log,
    StrippedDarkOakLog => impls::Log,
    StrippedAcaciaLog => impls::Log,
    StrippedJungleLog => impls::Log,

    Sand | RedSand | Gravel => impls::Falling,
  };
  out
}