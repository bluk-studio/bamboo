use crate::nbt::NBT;
use bb_macros::Transfer;

#[derive(Transfer, Debug, Clone, PartialEq)]
pub struct Item {
  pub id:     i32,
  pub count:  u8,
  // Only exists on 1.8-1.12 clients. 1.13+ clients use NBT for this
  pub damage: i16,
  pub nbt:    NBT,
}

impl Default for Item {
  fn default() -> Self { Item::new(0, 0, 0, NBT::default()) }
}

impl Item {
  pub fn new(id: i32, count: u8, damage: i16, nbt: NBT) -> Self { Item { id, count, damage, nbt } }

  pub fn id(&self) -> i32 { self.id }
  pub fn count(&self) -> u8 { self.count }
  pub fn nbt(&self) -> &NBT { &self.nbt }

  pub fn into_parts(self) -> (i32, u8, NBT) { (self.id, self.count, self.nbt) }
}
