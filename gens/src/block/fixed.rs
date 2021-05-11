// This handles loading all block versions 1.8-1.12
use serde_derive::Deserialize;
use std::{collections::HashMap, io};

use super::{Block, BlockVersion, State};

#[derive(Default, Debug, Deserialize)]
struct JsonVariation {
  metadata:     u32,
  #[serde(alias = "displayName")]
  display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonDropId {
  ID(u32),
  Meta { id: u32, metadata: u32 },
}

#[derive(Debug, Deserialize)]
struct JsonDrop {
  drop: JsonDropId,

  // These are item counts, or percentage changes if they are not an even int
  #[serde(alias = "minCount")]
  min_count: Option<f32>,
  #[serde(alias = "maxCount")]
  max_count: Option<f32>,
}

#[derive(Default, Debug, Deserialize)]
struct JsonBlock {
  id:           u32,
  #[serde(alias = "displayName")]
  display_name: String,
  name:         String,
  // If this is None or 0, then it is unbreakable
  hardness:     Option<f32>,
  variations:   Option<Vec<JsonVariation>>,
  // Vec of item ids
  drops:        Vec<JsonDrop>,
  diggable:     bool,
  transparent:  bool,
  #[serde(alias = "filterLight")]
  filter_light: u32,
  #[serde(alias = "emitLight")]
  emit_light:   u32,
  #[serde(alias = "boundingBox")]
  bounding_box: String,
  #[serde(alias = "stackSize")]
  stack_size:   u32,
  resistance:   f32,
  material:     Option<String>,
}

pub(super) fn load_data(file: &str) -> io::Result<BlockVersion> {
  let data: Vec<JsonBlock> = serde_json::from_str(file)?;
  let mut ver = BlockVersion { blocks: vec![] };
  for b in data {
    dbg!(&b);
    let state = b.id << 4;
    ver.blocks.push(Block {
      states:        b
        .variations
        .unwrap_or_else(Vec::new)
        .iter()
        .map(|s| State { id: state | s.metadata, properties: HashMap::new() })
        .collect(),
      name:          b.name,
      // Need a state id. In old versions, this is always the block id << 4 (the 4 bits are used
      // for metadata)
      id:            state,
      default_index: 0,
    });
  }
  Ok(ver)
}
