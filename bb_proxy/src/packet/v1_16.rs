use super::TypeConverter;
use crate::gnet::cb::Packet;
use bb_common::{
  chunk::paletted::Section,
  math::ChunkPos,
  nbt::{Tag, NBT},
  util::Buffer,
  version::BlockVersion,
};

// CHANGES:
// Biomes are now a length prefixed varint array, instead of an int array.
pub fn chunk(
  pos: ChunkPos,
  full: bool,
  bit_map: u16,
  sections: &[Section],
  conv: &TypeConverter,
) -> Packet {
  let biomes = full;
  let _skylight = true; // Assume overworld

  let mut chunk_data = vec![];
  let mut chunk_buf = Buffer::new(&mut chunk_data);
  for s in sections {
    chunk_buf.write_u16(s.non_air_blocks() as u16);
    chunk_buf.write_u8(s.data().bpe() as u8);
    if s.data().bpe() <= 8 {
      chunk_buf.write_varint(s.palette().len() as i32);
      for g in s.palette() {
        chunk_buf.write_varint(conv.block_to_old(*g as u32, BlockVersion::V1_16) as i32);
      }
    }
    let longs = s.data().long_array();
    chunk_buf.write_varint(longs.len() as i32);
    longs.iter().for_each(|v| chunk_buf.write_buf(&v.to_be_bytes()));
  }

  let mut biome_data = vec![];
  let mut biome_buf = Buffer::new(&mut biome_data);
  if biomes {
    biome_buf.write_varint(1024); // Length of biomes
    for _ in 0..1024 {
      // The first biome in the list of biomes in the dimension codec
      biome_buf.write_varint(0);
    }
  }

  let heightmap = vec![];
  let heightmap = NBT::new("", Tag::compound(&[("MOTION_BLOCKING", Tag::LongArray(heightmap))]));

  let mut data = Vec::with_capacity(chunk_buf.len());
  let mut buf = Buffer::new(&mut data);
  buf.write_buf(&heightmap.serialize());
  buf.write_buf(&biome_data);
  buf.write_varint(chunk_buf.len() as i32);
  buf.write_buf(&chunk_data);
  buf.write_varint(0); // No block entities

  Packet::ChunkDataV14 {
    chunk_x:                pos.x(),
    chunk_z:                pos.z(),
    is_full_chunk:          full,
    vertical_strip_bitmask: bit_map.into(),
    unknown:                data,
  }
}