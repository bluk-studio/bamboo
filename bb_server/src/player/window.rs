use crate::{
  item::{SharedInventory, Stack},
  player::ConnSender,
  world::WorldManager,
};
use bb_common::util::UUID;
use bb_server_macros::Window;
use std::sync::Arc;

trait WindowData {
  fn sync(&self, index: u32);
  fn access<F, R>(&self, index: u32, f: F) -> Option<R>
  where
    F: FnOnce(&Stack) -> R;
  fn access_mut<F, R>(&mut self, index: u32, f: F) -> Option<R>
  where
    F: FnOnce(&mut Stack) -> R;
  fn size(&self) -> u32;
  fn add(&mut self, stack: Stack) -> u8;
  fn open(&self, id: UUID, conn: &ConnSender);
  fn close(&self, id: UUID);
}

trait WindowHandler {
  fn on_update(&self, clicked: Option<u32>) { let _ = clicked; }
}

#[derive(Window, Debug, Clone)]
pub struct GenericWindow<const N: usize> {
  pub inv: SharedInventory<N>,
}

#[derive(Window, Debug, Clone)]
pub struct SmeltingWindow {
  pub input:  SharedInventory<1>,
  // #[filter(fuel)]
  pub fuel:   SharedInventory<1>,
  #[output]
  pub output: SharedInventory<1>,
}

#[derive(Window, Debug, Clone)]
pub struct CraftingWindow {
  #[output]
  pub output: SharedInventory<1>,
  pub grid:   SharedInventory<9>,
  #[not_inv]
  pub wm:     Arc<WorldManager>,
}

impl<const N: usize> WindowHandler for GenericWindow<N> {}
impl WindowHandler for SmeltingWindow {}

impl WindowHandler for CraftingWindow {
  fn on_update(&self, clicked: Option<u32>) {
    if let Some(clicked) = clicked {
      if clicked == 0 && self.output.lock().get(0).unwrap().is_empty() {
        let mut lock = self.grid.lock();
        for i in 0..9 {
          lock.set(i, Stack::empty());
        }
        return;
      }
    }
    if let Some(stack) = self.wm.json_data().crafting.craft(&self.grid.lock().inv) {
      self.output.lock().set(0, stack);
    } else {
      self.output.lock().set(0, Stack::empty());
    }
  }
}

#[derive(bb_server_macros::WindowEnum, Debug, Clone)]
pub enum Window {
  #[name("minecraft:generic_9x1")]
  Generic9x1(GenericWindow<9>),
  #[name("minecraft:generic_9x2")]
  Generic9x2(GenericWindow<18>),
  #[name("minecraft:generic_9x3")]
  Generic9x3(GenericWindow<27>),
  #[name("minecraft:generic_9x4")]
  Generic9x4(GenericWindow<36>),
  #[name("minecraft:generic_9x5")]
  Generic9x5(GenericWindow<45>),
  #[name("minecraft:generic_9x6")]
  Generic9x6(GenericWindow<54>),
  #[name("minecraft:generic_3x3")]
  Generic3x3(GenericWindow<9>),
  #[name("minecraft:crafting")]
  Crafting(CraftingWindow),
  /*
  #[name("minecraft:anvil")]
  Anvil(Anvil),
  #[name("minecraft:beacon")]
  Beacon(inv: SharedInventory<1>),
  #[name("minecraft:blast_furnace")]
  BlastFurnace(SmeltingWindow),
  #[name("minecraft:brewing_stand")]
  BrewingStand {
    bottles:    SharedInventory<3>,
    ingredient: SharedInventory<1>,
    fuel:       SharedInventory<1>,
  },
  #[name("minecraft:enchantment")]
  Enchantment { book: SharedInventory<1>, lapis: SharedInventory<1> },
  #[name("minecraft:furnace")]
  Furnace {
    input:  SharedInventory<1>,
    #[filter(fuel)]
    fuel:   SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:grindstone")]
  Grindstone {
    inputs: SharedInventory<2>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:hopper")]
  Hopper { inv: SharedInventory<5> },
  #[name("minecraft:lectern")]
  Lectern { book: SharedInventory<1> },
  #[name("minecraft:loom")]
  Loom {
    banner:  SharedInventory<1>,
    dye:     SharedInventory<1>,
    pattern: SharedInventory<1>,
    #[output]
    output:  SharedInventory<1>,
  },
  #[name("minecraft:merchant")]
  Merchant { inv: SharedInventory<1> },
  #[name("minecraft:shulker_box")]
  ShulkerBox { inv: SharedInventory<27> },
  #[name("minecraft:smithing")]
  Smithing {
    input:   SharedInventory<1>,
    upgrade: SharedInventory<1>,
    #[output]
    output:  SharedInventory<1>,
  },
  #[name("minecraft:smoker")]
  Smoker {
    input:  SharedInventory<1>,
    #[filter(fuel)]
    fuel:   SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:cartography")]
  Cartography {
    map:    SharedInventory<1>,
    paper:  SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:stonecutter")]
  Stonecutter {
    input:  SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  */
}

pub struct ItemsIter<'a> {
  win:   &'a Window,
  index: u32,
}

impl Iterator for ItemsIter<'_> {
  type Item = Stack;

  fn next(&mut self) -> Option<Self::Item> {
    self.win.get(self.index).map(|it| {
      self.index += 1;
      it
    })
  }
}

impl Window {
  pub fn get(&self, index: u32) -> Option<Stack> { self.access(index, |s| s.clone()) }
  pub fn set(&mut self, index: u32, stack: Stack) {
    self.access_mut(index, move |s| *s = stack);
    self.sync(index);
  }
  pub fn items(&self) -> ItemsIter<'_> { ItemsIter { win: self, index: 0 } }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_window_size() {
    let win = GenericWindow::<0> { inv: SharedInventory::new() };
    assert_eq!(win.size(), 0);
    let win = GenericWindow::<3> { inv: SharedInventory::new() };
    assert_eq!(win.size(), 3);
    let win = GenericWindow::<1234> { inv: SharedInventory::new() };
    assert_eq!(win.size(), 1234);

    let win = GenericWindow::<3> { inv: SharedInventory::new() };
    assert_eq!(win.access(0, |it| it.clone()), Some(Stack::empty()));
    assert_eq!(win.access(1, |it| it.clone()), Some(Stack::empty()));
    assert_eq!(win.access(2, |it| it.clone()), Some(Stack::empty()));
    assert_eq!(win.access(3, |it| it.clone()), None);
  }
}
