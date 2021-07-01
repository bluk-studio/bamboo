use super::section::Section as ChunkSection;

use crate::{
  math::{Pos, PosError},
  proto,
};
use std::{collections::HashMap, convert::TryFrom};

mod bits;
#[cfg(test)]
mod tests;

use bits::BitArray;

#[derive(Debug)]
pub struct Section {
  bits_per_block:  u8,
  data:            Vec<u64>,
  // Each index into palette is a palette id. The values are global ids.
  palette:         Vec<u32>,
  // Each index is a palette id, and the sum of this array must always be 4096 (16 * 16 * 16).
  block_amounts:   Vec<u32>,
  // This maps global ids to palette ids.
  reverse_palette: HashMap<u32, u32>,
}

impl Default for Section {
  fn default() -> Self {
    let mut reverse_palette = HashMap::new();
    reverse_palette.insert(0, 0);
    Section {
      bits_per_block: 4,
      // Number of blocks times bits per block divided by sizeof(u64)
      data: vec![0; 16 * 16 * 16 * 4 / 64],
      palette: vec![0],
      block_amounts: vec![4096],
      reverse_palette,
    }
  }
}

impl Section {
  pub(super) fn new() -> Box<Self> {
    Box::new(Self::default())
  }
  fn validate_proto(pb: &proto::chunk::Section) {
    if pb.bits_per_block < 4 || pb.bits_per_block > 64 {
      panic!("invalid bits per block recieved from proto: {}", pb.bits_per_block);
    }
    if pb.palette.len() > 256 {
      panic!("got a palette that was too long: {} > 256", pb.palette.len());
    }
    if let Some(&v) = pb.palette.get(0) {
      if v != 0 {
        panic!("the first element of the palette must be 0, got {}", v);
      }
    }
    if pb.data.len() != 16 * 16 * 16 * pb.bits_per_block as usize / 64 {
      panic!(
        "protobuf data length is incorrect. got {} longs, expected {} longs",
        pb.data.len(),
        16 * 16 * 16 * pb.bits_per_block as usize / 64
      );
    }
  }
  pub(super) fn from_latest_proto(pb: proto::chunk::Section) -> Box<Self> {
    Section::validate_proto(&pb);
    let mut chunk = Section {
      bits_per_block:  pb.bits_per_block as u8,
      data:            pb.data,
      block_amounts:   vec![0; pb.palette.len()],
      palette:         pb.palette,
      reverse_palette: HashMap::new(),
    };
    for (i, &v) in chunk.palette.iter().enumerate() {
      chunk.reverse_palette.insert(v, i as u32);
    }
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          let val = chunk.get_palette(Pos::new(x, y, z));
          chunk.block_amounts[val as usize] += 1;
        }
      }
    }
    Box::new(chunk)
  }
  /// Creates a chunk section from the given protobuf. The function `f` will be
  /// used to convert the block ids within the protobuf section into the block
  /// ids that should be used within the new chunk section.
  ///
  /// Currently, we assume that after converting ids, the new ids will be in the
  /// same order as the old ones.
  pub(super) fn from_old_proto(pb: proto::chunk::Section, f: &dyn Fn(u32) -> u32) -> Box<Self> {
    Section::validate_proto(&pb);
    let mut chunk = Section {
      bits_per_block:  pb.bits_per_block as u8,
      data:            pb.data,
      block_amounts:   vec![0; pb.palette.len()],
      palette:         pb.palette.into_iter().map(f).collect(),
      reverse_palette: HashMap::new(),
    };
    for (i, &v) in chunk.palette.iter().enumerate() {
      chunk.reverse_palette.insert(v, i as u32);
    }
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          let val = chunk.get_palette(Pos::new(x, y, z));
          chunk.block_amounts[val as usize] += 1;
        }
      }
    }
    Box::new(chunk)
  }
  #[inline(always)]
  fn index(&self, pos: Pos) -> (usize, usize, usize) {
    let index = (pos.y() << 8 | pos.z() << 4 | pos.x()) as usize;
    let bpb = self.bits_per_block as usize;
    let bit_index = index * bpb;
    let first = bit_index / 64;
    let second = (bit_index + bpb - 1) / 64;
    let shift = bit_index % 64;
    (first, second, shift)
  }
  /// Writes a single palette id into self.data.
  fn set_palette(&mut self, pos: Pos, id: u32) {
    let (first, second, shift) = self.index(pos);
    let bpb = self.bits_per_block as usize;
    #[cfg(debug_assertions)]
    if id >= 1 << bpb {
      panic!("passed invalid id {} (must be within 0..{})", id, 1 << bpb);
    }
    let mask = (1 << bpb) - 1;
    if first == second {
      // Clear the bits of the new id
      self.data[first] &= !(mask << shift);
      // Set the new id
      self.data[first] |= (id as u64) << shift;
    } else {
      let second_shift = 64 - shift;
      // Clear the bits of the new id
      self.data[first] &= !(mask << shift);
      self.data[second] &= !(mask >> second_shift);
      // Set the new id
      // TODO: All of these shifts will break on 5+ bits per block. Need to fix.
      self.data[first] |= (id as u64) << shift;
      self.data[second] |= (id as u64) >> second_shift;
    }
  }
  /// Returns the palette id at the given position. This only reads from
  /// `self.data`.
  fn get_palette(&self, pos: Pos) -> u32 {
    let (first, second, shift) = self.index(pos);
    let bpb = self.bits_per_block as usize;
    let mask = (1 << bpb) - 1;
    let val = if first == second {
      // Get the id from data
      (self.data[first] >> shift) & mask
    } else {
      let second_shift = 64 - shift;
      // Get the id from the two values
      (self.data[first] >> shift) & mask | (self.data[second] << second_shift) & mask
    };

    #[cfg(not(debug_assertions))]
    let v = val as u32;
    #[cfg(debug_assertions)]
    let v = u32::try_from(val).unwrap();
    v
  }
  /// This adds a new item to the palette. It will shift all block data, and
  /// extend bits per block (if needed). It will also update the palettes, and
  /// shift the block amounts around. It will not modify the actual amounts in
  /// block_amounts, only the position of each amount. It will insert a 0 into
  /// block_amounts at the index returned. `ty` must not already be in the
  /// palette. Returns the new palette id.
  fn insert(&mut self, ty: u32) -> u32 {
    if self.palette.len() + 1 >= 1 << self.bits_per_block as usize {
      self.increase_bits_per_block();
    }
    let mut palette_id = self.palette.len() as u32;
    for (i, g) in self.palette.iter().enumerate() {
      if *g > ty {
        palette_id = i as u32;
        break;
      }
    }
    self.palette.insert(palette_id as usize, ty);
    // We add to this in set_block, not here
    self.block_amounts.insert(palette_id as usize, 0);
    for (_, p) in self.reverse_palette.iter_mut() {
      if *p >= palette_id {
        *p += 1;
      }
    }
    self.reverse_palette.insert(ty, palette_id);
    self.shift_all_above(palette_id, 1);
    palette_id
  }
  /// This removes the given palette id from the palette. This includes
  /// modifying the block_amounts array. It will not affect any of the values in
  /// block_amounts, but it will shift the values over if needed. It will also
  /// decrease the bits per block if needed. `id` must be a valid index into
  /// the palette.
  fn remove(&mut self, id: u32) {
    // if self.palette.len() - 1 < 1 << (self.bits_per_block as usize - 1) {
    //   self.decrease_bits_per_block();
    // }
    let ty = self.palette[id as usize];
    self.palette.remove(id as usize);
    self.block_amounts.remove(id as usize);
    for (_, p) in self.reverse_palette.iter_mut() {
      if *p > id {
        *p -= 1;
      }
    }
    self.reverse_palette.remove(&ty);
    self.shift_all_above(id, -1);
  }
  /// This shifts all values in self.data by the given shift value. To clarify,
  /// this just adds shift_amount. It does not bitshift. Used after the palette
  /// has been modified. This also checks if each block id is `>= id`, not `>
  /// id`.
  fn shift_all_above(&mut self, id: u32, shift_amount: i32) {
    let bpb = self.bits_per_block as usize;
    let mut bit_index = 0;
    let mask = (1 << bpb) - 1;
    for _ in 0..16 {
      for _ in 0..16 {
        for _ in 0..16 {
          // Manual implementation of get_palette and set_palette, as calling index()
          // would be slower.
          let first = bit_index / 64;
          let second = (bit_index + bpb - 1) / 64;
          let shift = bit_index % 64;
          let mut val = if first == second {
            // Get the id from data
            self.data[first] >> shift & mask
          } else {
            let second_shift = 64 - shift;
            // Get the id from the two values
            (self.data[first] >> shift) & mask | (self.data[second] << second_shift) & mask
          } as i32;
          if val as u32 >= id {
            val += shift_amount;
            let val = val as u64;
            if first == second {
              // Clear the bits of the new id
              self.data[first] &= !(mask << shift);
              // Set the new id
              self.data[first] |= val << shift;
            } else {
              let second_shift = 64 - shift;
              // Clear the bits of the new id
              self.data[first] &= !(mask << shift);
              self.data[second] &= !(mask >> second_shift);
              // Set the new id
              self.data[first] |= val << shift;
              self.data[second] |= val >> second_shift;
            }
          }
          bit_index += bpb;
        }
      }
    }
  }
  /// Increases the bits per block by one. This will increase
  /// self.bits_per_block, and update the long array. It does not affect the
  /// palette at all.
  fn increase_bits_per_block(&mut self) {
    // Very Bad Things can happen here, lets just call it all unsafe
    unsafe {
      let bpb = (self.bits_per_block + 1) as usize;
      let mut new_data = vec![0; 16 * 16 * 16 * bpb / 64];
      let mut bit_index = 0;
      let mask = (1 << bpb) - 1;
      for y in 0..16 {
        for z in 0..16 {
          for x in 0..16 {
            // SAFETY: We know that new_data will have enugh bits to fit 4096
            // numbers that are `bpb` bits long. So, diving bit_index by 64
            // will always give us a valid index into the vector.
            let first = bit_index / 64;
            // SAFETY: Again, we know that bit_index will always fit into the vector.
            // This number will always point to the last bit of the new value.
            let second = (bit_index + bpb - 1) / 64;
            let shift = bit_index % 64;
            let id = self.get_palette(Pos::new(x, y, z));
            if first == second {
              // Clear the bits of the new id
              let f = new_data.get_unchecked_mut(first);
              *f &= !(mask << shift);
              // Set the new id
              let s = new_data.get_unchecked_mut(second);
              *s |= (id as u64) << shift;
            } else {
              // We have a situation where we want to place a number, but it is
              // going to be split between two longs.
              //
              // A = v << shift;
              // B = v >> (64 - shift);
              //
              //         B B | A A A
              // 2 2 2 2 2 2 | 1 1 1 1 1 1
              // (higher bits are on the left, lower bits are on the right)
              //
              // So shift will move the number to the left, leaving us with the
              // lower bits. `64 - shift` will give us a small number, which we
              // can use to/ shift the number to the right. This resulting number
              // will only have the higher bits, which is what we want.

              // Clear the low bits of the new id
              let f = new_data.get_unchecked_mut(first);
              *f &= !(mask << shift);
              // Set the new id
              *f |= (id as u64) << shift;

              let second_shift = 64 - shift;
              // Clear the high bits of the new id
              let s = new_data.get_unchecked_mut(second);
              *s &= !(mask >> second_shift);
              // Set the new id
              *s |= (id as u64) >> second_shift;
            }
            bit_index += bpb;
          }
        }
      }
      self.data = new_data;
      self.bits_per_block += 1;
    }
  }
}

impl ChunkSection for Section {
  fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    // Currently, this function is almost as fast as it could be. The one limiting
    // factor is with replacing a single unique block with another unique block. If
    // that moves the block data over the bits_per_block threshhold, all of the data
    // will be copied twice. It will also insert and remove from the palette at the
    // same index.
    //
    // This is less than optimal, to say the least. However, the only way this could
    // happen ingame is with a /setblock. This will not happen at all with
    // breaking/placing blocks, as air will always be in the palette. So in
    // survival, this will never come up.
    let mut prev = self.get_palette(pos);
    let palette_id = match self.reverse_palette.get(&ty) {
      Some(&palette_id) => {
        if prev == palette_id {
          // The same block is being placed, so we do nothing.
          return Ok(());
        }
        self.set_palette(pos, palette_id);
        palette_id
      }
      None => {
        let palette_id = self.insert(ty);
        // If insert() was called, and it inserted before prev, the block_amounts would
        // have been shifted, and prev needs to be shifted as well.
        if palette_id <= prev {
          prev += 1;
        }
        self.set_palette(pos, palette_id);
        palette_id
      }
    };
    self.block_amounts[palette_id as usize] += 1;
    self.block_amounts[prev as usize] -= 1;
    if self.block_amounts[prev as usize] == 0 && prev != 0 {
      self.remove(prev);
    }
    Ok(())
  }
  fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    if min == Pos::new(0, 0, 0) && max == Pos::new(15, 15, 15) {
      // Simple case. We get to just replace the whole section.
      if ty == 0 {
        // With air, this is even easier.
        *self = Section::default();
      } else {
        // With anything else, we need to make sure air stays in the palette.
        *self = Section {
          data: vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64],
          palette: vec![0, ty],
          reverse_palette: vec![(0, 0), (ty, 1)].iter().cloned().collect(),
          block_amounts: vec![0, 4096],
          ..Default::default()
        };
      }
    } else {
      // More difficult case. Here, we need to modify all the block amounts,
      // then remove all the items we need to from the palette. Then, we add the
      // new item to the palette, and update the block data.
      for y in min.y()..=max.y() {
        for z in min.z()..=max.z() {
          for x in min.x()..=max.x() {
            let id = self.get_palette(Pos::new(x, y, z));
            let amt = self.block_amounts[id as usize];
            // Debug assertions mean that we cannot subtract with overflow here.
            self.block_amounts[id as usize] = amt - 1;
          }
        }
      }
      let mut ids_to_remove = vec![];
      for (id, amt) in self.block_amounts.iter().enumerate() {
        #[cfg(debug_assertions)]
        if *amt > 4096 {
          dbg!(&self);
          unreachable!("amount is invalid! should not be possible")
        }
        // Make sure we do not remove air from the palette.
        if *amt == 0 && id != 0 {
          ids_to_remove.push(id as u32);
        }
      }
      for id in ids_to_remove {
        self.remove(id);
      }
      let palette_id = match self.reverse_palette.get(&ty) {
        Some(&palette_id) => palette_id,
        None => self.insert(ty),
      };
      self.block_amounts[palette_id as usize] +=
        ((max.x() - min.x() + 1) * (max.y() - min.y() + 1) * (max.z() - min.z() + 1)) as u32;
      for y in min.y()..=max.y() {
        for z in min.z()..=max.z() {
          for x in min.x()..=max.x() {
            self.set_palette(Pos::new(x, y, z), palette_id);
          }
        }
      }
    }
    Ok(())
  }
  fn get_block(&self, pos: Pos) -> Result<u32, PosError> {
    let id = self.get_palette(pos);
    Ok(self.palette[id as usize])
  }
  fn duplicate(&self) -> Box<dyn ChunkSection + Send> {
    Box::new(Section {
      bits_per_block:  self.bits_per_block,
      data:            self.data.clone(),
      palette:         self.palette.clone(),
      block_amounts:   self.block_amounts.clone(),
      reverse_palette: self.reverse_palette.clone(),
    })
  }
  fn to_latest_proto(&self) -> proto::chunk::Section {
    proto::chunk::Section {
      palette:        self.palette.clone(),
      bits_per_block: self.bits_per_block.into(),
      non_air_blocks: (4096 - self.block_amounts[0]) as i32,
      data:           self.data.clone(),
    }
  }
  fn to_old_proto(&self, f: &dyn Fn(u32) -> u32) -> proto::chunk::Section {
    proto::chunk::Section {
      palette:        self.palette.iter().map(|v| f(*v)).collect(),
      bits_per_block: self.bits_per_block.into(),
      non_air_blocks: (4096 - self.block_amounts[0]) as i32,
      data:           self.data.clone(),
    }
  }
}