use super::{ser, VersionConverter};
use crate::{
  chunk::paletted::Section,
  math::{ChunkPos, Pos},
  util::Buffer,
  version::ProtocolVersion,
};
use sc_generated::net::cb::Packet as GPacket;
use std::{error::Error, fmt};

#[derive(Debug, Clone, sc_macros::Packet)]
pub enum Packet {
  BlockUpdate {
    pos:   Pos,
    state: u32,
  },
  Chat {
    msg: String,
    ty:  u8,
  },
  Chunk {
    pos:      ChunkPos,
    bit_map:  u16,
    sections: Vec<Section>,
  },
  EntityVelocity {
    eid: i32,
    x:   i16,
    y:   i16,
    z:   i16,
  },
  JoinGame {
    eid:                i32,
    hardcore_mode:      bool,
    game_mode:          u8,
    dimension:          i8,
    level_type:         String,
    difficulty:         i8,
    view_distance:      u16,
    reduced_debug_info: bool,
  },
  KeepAlive {
    id: u32,
  },
  PlayerHeader {
    header: String,
    footer: String,
  },
  SetPosLook {
    x:           f64,
    y:           f64,
    z:           f64,
    yaw:         f32,
    pitch:       f32,
    flags:       u8,
    teleport_id: u32,
  },
  UnloadChunk {
    pos: ChunkPos,
  },
  UpdateViewPos {
    pos: ChunkPos,
  },
}

#[derive(Debug, Clone)]
pub enum WriteError {
  InvalidVer,
}

impl fmt::Display for WriteError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::InvalidVer => write!(f, "invalid version"),
    }
  }
}

impl Error for WriteError {}

impl Packet {
  pub fn to_tcp(
    self,
    ver: ProtocolVersion,
    conv: &impl VersionConverter,
  ) -> Result<GPacket, WriteError> {
    Ok(match self {
      // Packet::Chunk { .. } => GPacket::ChunkDataV8 {},
      Packet::Chat { msg, ty } => {
        if ver < ProtocolVersion::V1_12_2 {
          GPacket::ChatV8 { chat_component: msg, ty }
        } else {
          GPacket::ChatV12 { chat_component: msg, ty: None, unknown: vec![ty] }
        }
      }
      Packet::Chunk { pos, bit_map, sections } => ser::chunk(pos, bit_map, sections, ver, conv),
      Packet::JoinGame {
        eid,
        hardcore_mode,
        game_mode,
        dimension,
        level_type,
        difficulty,
        view_distance,
        reduced_debug_info,
      } => {
        let mut out = Buffer::new(vec![]);
        out.write_u8(game_mode);
        if ver >= ProtocolVersion::V1_9_2 {
          out.write_i32(dimension.into());
        } else {
          out.write_i8(dimension.into());
        }
        out.write_i8(difficulty);
        if ver <= ProtocolVersion::V1_12_2 {
          // Max players. Ignored on the versions where its present.
          out.write_u8(0);
        }
        out.write_str(&level_type);
        if ver >= ProtocolVersion::V1_14_4 {
          out.write_varint(view_distance.into());
        }
        out.write_bool(reduced_debug_info);

        GPacket::JoinGameV8 {
          entity_id: eid,
          hardcore_mode,
          game_type: None,
          dimension: None,
          difficulty: None,
          max_players: None,
          world_type: None,
          reduced_debug_info: None,
          unknown: out.into_inner(),
        }
      }
      Packet::KeepAlive { id } => {
        if ver < ProtocolVersion::V1_12_2 {
          GPacket::KeepAliveV8 { id: id as i32 }
        } else {
          GPacket::KeepAliveV12 { id: id.into() }
        }
      }
      Packet::PlayerHeader { header, footer } => GPacket::PlayerListHeaderV8 { header, footer },
      Packet::SetPosLook { x, y, z, yaw, pitch, flags, teleport_id } => {
        let mut buf = Buffer::new(vec![]);
        buf.write_u8(flags);
        if ver >= ProtocolVersion::V1_9 {
          buf.write_varint(teleport_id as i32);
        }
        GPacket::PlayerPosLookV8 {
          x,
          y,
          z,
          yaw,
          pitch,
          field_179835_f: None,
          unknown: buf.into_inner(),
        }
      }
      Packet::UnloadChunk { pos } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::ChunkDataV8 {
            chunk_x:        pos.x(),
            chunk_z:        pos.z(),
            field_149279_g: true,
            extracted_data: None,
            // Zero bit mask, then zero length varint
            unknown:        vec![0, 0, 0],
          }
        } else {
          GPacket::UnloadChunkV9 { x: pos.x(), z: pos.z() }
        }
      }
      Packet::UpdateViewPos { pos } => {
        if ver >= ProtocolVersion::V1_14 {
          GPacket::ChunkRenderDistanceCenterV14 { chunk_x: pos.x(), chunk_z: pos.z() }
        } else {
          panic!("cannot send UpdateViewPos for version {}", ver);
        }
      }
      _ => todo!("convert {:?} into generated packet", self),
    })
  }
}
