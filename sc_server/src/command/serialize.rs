use super::{Command, CommandTree, NodeType, Parser, StringType};
use sc_common::{net::cb, util::Buffer};

impl NodeType {
  fn mask(&self) -> u8 {
    match self {
      Self::Root => 0x00,
      Self::Literal => 0x01,
      Self::Argument(_) => 0x02,
    }
  }
}

struct IndexNode {
  name:     String,
  ty:       NodeType,
  children: Vec<usize>,
}

impl CommandTree {
  /// Serializes the entire command tree. This will be called any time a player
  /// joins.
  pub async fn serialize(&self) -> cb::Packet {
    // This is a reverse-order list of all the nodes. The highest level node (the
    // root node) will be last.
    let mut nodes = vec![];

    let commands = self.commands.lock().await;
    let c = Command {
      name:     "".into(),
      ty:       NodeType::Root,
      children: commands.values().map(|(command, _)| command.clone()).collect(),
    };
    c.write_nodes(&mut nodes);

    let mut data = Buffer::new(vec![]);
    data.write_varint(nodes.len() as i32);

    for node in &nodes {
      let mask = node.ty.mask();
      // TODO: Check executable bits
      data.write_u8(mask);
      data.write_varint(node.children.len() as i32);
      for &index in &node.children {
        data.write_varint(index as i32);
      }
      match &node.ty {
        NodeType::Argument(parser) => {
          data.write_str(&node.name);
          data.write_str(&parser.name());
          parser.write_data(&mut data);
        }
        NodeType::Literal => {
          data.write_str(&node.name);
        }
        NodeType::Root => {}
      }
    }

    cb::Packet::DeclareCommands {
      nodes_v1_13:      Some(data.into_inner()),
      root_index_v1_13: Some((nodes.len() - 1) as i32),
    }
  }
}

impl Command {
  // Adds all children in order from the lowest nodes up. All dependencies must
  // already be in the list before a node can be written.
  //
  // Returns the index of self into the array.
  fn write_nodes(&self, nodes: &mut Vec<IndexNode>) -> usize {
    let children = self.children.iter().map(|c| c.write_nodes(nodes)).collect();
    nodes.push(IndexNode { name: self.name.clone(), ty: self.ty.clone(), children });
    nodes.len() - 1
  }
}

impl Parser {
  /// Returns the name of this parser. Used in packet serialization.
  #[rustfmt::skip]
  pub fn name(&self) -> &'static str {
    match self {
      Self::Bool               => "brigadier:bool",
      Self::Double { .. }      => "brigadier:double",
      Self::Float { .. }       => "brigadier:float",
      Self::Int { .. }         => "brigadier:int",
      Self::String(_)          => "brigadier:string",
      Self::Entity { .. }      => "minecraft:entity",
      Self::ScoreHolder { .. } => "minecraft:score_holder",
      Self::GameProfile        => "minecraft:game_profile",
      Self::BlockPos           => "minecraft:block_pos",
      Self::ColumnPos          => "minecraft:column_pos",
      Self::Vec3               => "minecraft:vec3",
      Self::Vec2               => "minecraft:vec2",
      Self::BlockState         => "minecraft:block_state",
      Self::BlockPredicate     => "minecraft:block_predicate",
      Self::ItemStack          => "minecraft:item_stack",
      Self::ItemPredicate      => "minecraft:item_predicate",
      Self::Color              => "minecraft:color",
      Self::Component          => "minecraft:component",
      Self::Message            => "minecraft:message",
      Self::Nbt                => "minecraft:nbt",
      Self::NbtPath            => "minecraft:nbt_path",
      Self::Objective          => "minecraft:objective",
      Self::ObjectiveCriteria  => "minecraft:objective_criteria",
      Self::Operation          => "minecraft:operation",
      Self::Particle           => "minecraft:particle",
      Self::Rotation           => "minecraft:rotation",
      Self::Angle              => "minecraft:angle",
      Self::ScoreboardSlot     => "minecraft:scoreboard_slot",
      Self::Swizzle            => "minecraft:swizzle",
      Self::Team               => "minecraft:team",
      Self::ItemSlot           => "minecraft:item_slot",
      Self::ResourceLocation   => "minecraft:resource_location",
      Self::MobEffect          => "minecraft:mob_effect",
      Self::Function           => "minecraft:function",
      Self::EntityAnchor       => "minecraft:entity_anchor",
      Self::Range { .. }       => "minecraft:range",
      Self::IntRange           => "minecraft:int_range",
      Self::FloatRange         => "minecraft:float_range",
      Self::ItemEnchantment    => "minecraft:item_enchantment",
      Self::EntitySummon       => "minecraft:entity_summon",
      Self::Dimension          => "minecraft:dimension",
      Self::Uuid               => "minecraft:uuid",
      Self::NbtTag             => "minecraft:nbt_tag",
      Self::NbtCompoundTag     => "minecraft:nbt_compound_tag",
      Self::Time               => "minecraft:time",
      Self::Modid              => "forge:modid",
      Self::Enum               => "forge:enum",
    }
  }

  /// If this parser stores any extra data, that will be written to the buffer.
  /// Most nodes will not write any extra data.
  pub fn write_data(&self, buf: &mut Buffer) {
    match self {
      Self::Double { min, max } => {
        let mut bitmask = 0;
        if min.is_some() {
          bitmask |= 0x01;
        }
        if max.is_some() {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
        if let Some(min) = min {
          buf.write_f64(*min);
        }
        if let Some(max) = max {
          buf.write_f64(*max);
        }
      }
      Self::Float { min, max } => {
        let mut bitmask = 0;
        if min.is_some() {
          bitmask |= 0x01;
        }
        if max.is_some() {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
        if let Some(min) = min {
          buf.write_f32(*min);
        }
        if let Some(max) = max {
          buf.write_f32(*max);
        }
      }
      Self::Int { min, max } => {
        let mut bitmask = 0;
        if min.is_some() {
          bitmask |= 0x01;
        }
        if max.is_some() {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
        if let Some(min) = min {
          buf.write_i32(*min);
        }
        if let Some(max) = max {
          buf.write_i32(*max);
        }
      }
      Self::String(ty) => {
        buf.write_varint(match ty {
          StringType::Word => 0,
          StringType::Quotable => 1,
          StringType::Greedy => 2,
        });
      }
      Self::Entity { single, players } => {
        let mut bitmask = 0;
        if *single {
          bitmask |= 0x01;
        }
        if *players {
          bitmask |= 0x02;
        }
        buf.write_u8(bitmask);
      }
      Self::ScoreHolder { multiple } => {
        let mut bitmask = 0;
        if *multiple {
          bitmask |= 0x01;
        }
        buf.write_u8(bitmask);
      }
      Self::Range { decimals } => {
        buf.write_bool(*decimals);
      }
      _ => {}
    }
  }
}