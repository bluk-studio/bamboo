use std::{
  collections::HashMap,
  io::{Error, ErrorKind},
};

use super::{utils, PacketSpec};

use common::{
  net::{cb, tcp, Other},
  util::Buffer,
};

pub(super) fn gen_spec() -> PacketSpec {
  let mut spec = PacketSpec { gens: HashMap::new() };
  spec.add(cb::ID::PlayerInfo, utils::generate_player_info);
  spec.add(cb::ID::UnloadChunk, |gen, v, p| {
    let mut out = tcp::Packet::new(gen.convert_id(v, cb::ID::MapChunk), v);
    out.write_i32(p.get_int("chunk_x")?);
    out.write_i32(p.get_int("chunk_z")?);
    out.write_bool(true); // Must be true to unload
    out.write_u16(0); // No chunks
    out.write_varint(0); // No data
    Ok(vec![out])
  });
  spec.add(cb::ID::MapChunk, |gen, v, p| {
    let mut out = tcp::Packet::new(gen.convert_id(v, cb::ID::MapChunk), v);
    // TODO: Error handling should be done within the packet.
    let chunk = match p.read_other().unwrap() {
      Other::Chunk(c) => c,
      o => return Err(Error::new(ErrorKind::InvalidData, format!("expected chunk, got {:?}", o))),
    };
    out.write_i32(chunk.x);
    out.write_i32(chunk.z);
    out.write_bool(true); // Always a new chunk

    let biomes = true; // Always true with new chunk set
    let skylight = true; // Assume overworld

    let mut bitmask = 0;
    for y in chunk.sections.keys() {
      bitmask |= 1 << y;
    }
    out.write_u16(bitmask);

    let mut buf = Buffer::new(vec![]);
    // Makes an ordered list of chunk sections
    let mut sections = vec![None; 16];
    for (y, s) in &chunk.sections {
      sections[*y as usize] = Some(s);
    }
    // Iterates through chunks in order, from ground up. flatten() skips all None
    // sections.
    let mut total_sections = 0;
    for s in sections.into_iter().flatten() {
      total_sections += 1;
      // These are little endian. I don't know why. It probably has something to do
      // with the way I serialize things, but I couldn't really be bothered to figure
      // it out (because it works).
      buf.write_buf(&s.data.iter().map(|v| v.to_le_bytes()).flatten().collect::<Vec<u8>>());
    }
    // Light data
    for _ in 0..total_sections * 16 * 16 * 16 / 2 {
      // Each lighting value is 1/2 byte
      buf.write_u8(0xff);
    }
    if skylight {
      for _ in 0..total_sections * 16 * 16 * 16 / 2 {
        // Each lighting value is 1/2 byte
        buf.write_u8(0xff);
      }
    }
    if biomes {
      for _ in 0..256 {
        buf.write_u8(127); // Void biome
      }
    }

    // Not needed. Leaving commented out for reference
    //
    // expected := num_sections * 16*16*16 * 2 // Block data
    // expected += num_sections * 16*16*16 / 2 // Block light data
    // if skylight {
    //   expected += num_sections * 16*16*16 / 2 // Sky light data
    // }
    // if biomes {
    //   expected += 256 // Biome data
    // }
    // if buf.Length() != int32(expected) {
    //   fmt.Println("ERROR: Incorrectly serialized chunk! Expected length:",
    // expected, "actual length:", buf.Length()) }
    out.write_varint(buf.len() as i32);
    out.write_buf(&buf);

    Ok(vec![out])
  });
  spec
}
