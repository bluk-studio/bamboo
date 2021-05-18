use std::collections::HashMap;

use super::{data, data::Data, Kind};

use common::version::BlockVersion;

/// This is a version converter. It is how all block ids are converter between
/// versions. At compile time, all of the vanilla Minecraft versions that are
/// supported are processed and converted into various files. One of these files
/// is a csv file, which is embedded into this binary. This csv contains a list
/// of all modern blocks within the game, and the ids that those ids map to in
/// all older versions of the game. This file is quite large, and is parsed any
/// time one of these is created. This type ends up using about 600K of memory,
/// so you should only ever create one. The
/// [`WorldManager`](crate::world::WorldManager) has one of these which you can
/// use.
///
/// This type does not implement [`Default`], because it is very expensive to
/// create one of these. A new one should only be constructed explicitly, and
/// should never be neccessary.
pub struct TypeConverter {
  // Each index into the outer vec is a kind id. Indexing into the inner vec is each variant of the
  // given kind. These are in such an order that iterating through both of them will get all block
  // types in the same order as the global palette of the latest version.
  kinds:    Vec<Data>,
  versions: Vec<Version>,
}

impl TypeConverter {
  /// Creates a new converter. This will parse the csv file, and allocate around
  /// 600K of memory. Do not call this unless you have a very good reason.
  /// Instead, use
  /// [`WorldManager::get_converter`](crate::world::WorldManager::
  /// get_converter).
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self { kinds: data::generate_kinds(), versions: generate_versions() }
  }

  /// Takes the given old block id, which is part of `ver`, and returns the new
  /// id that it maps to. If the id is invalid, this will return 0 (air).
  pub fn to_latest(&self, id: u32, ver: BlockVersion) -> u32 {
    match self.versions[ver.to_index() as usize].to_new.get(&id) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Takes the new block id, and converts it to the old id, for the given
  /// version. If the id is invalid, this will return 0 (air).
  pub fn to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.versions[ver.to_index() as usize - 1].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Gets all the data for a given block kind. This includes all the types, the
  /// default type, and state ids.
  pub fn get(&self, k: Kind) -> &Data {
    &self.kinds[k.to_u32() as usize]
  }
}

/// This is the conversion table for a single old version of the game and the
/// latest version. This includes a list of old ids, whose index is the new
/// block id. It also contains a HashMap, which is used to convert old ids into
/// new ones. This might not be fastest or most memory efficient way, but it is
/// certainly the easiest. Especially before 1.13, block ids are very sparse,
/// and a HashMap will be the best option.
#[derive(Debug)]
pub struct Version {
  to_old: Vec<u32>,
  to_new: HashMap<u32, u32>,
  ver:    BlockVersion,
}

/// Generates a table from all block kinds to any block data that kind has. This
/// does not include cross-versioning data. This includes information like the
/// block states, the properties it might have, and custom handlers for when the
/// block is place (things like making fences connect, or making stairs rotate
/// correctly).
///
/// This should only be called once, and will be done internally in the
/// [`WorldManager`](crate::world::WorldManager). This is left public as it may
/// be moved to a seperate crate in the future, as it takes a long time to
/// generate the source files for this.
///
/// Most of this function is generated at compile time. See
/// `gens/src/block/mod.rs` and `build.rs` for more.
///
/// This Vec<Version> is in order of block versions. Use
/// BlockVersion::from_index() and BlockVersion::to_index() to convert between
/// indicies and block versions.
pub fn generate_versions() -> Vec<Version> {
  let mut versions = vec![];
  let csv = include_str!(concat!(env!("OUT_DIR"), "/block/versions.csv"));
  for (i, l) in csv.lines().enumerate() {
    let mut sections = l.split(',').enumerate();
    // Remove the first element. This is the latest block id, which will always be
    // the same as i.
    sections.next().unwrap();
    if i == 0 {
      for (j, _) in sections {
        let ver = BlockVersion::from_index_rev(j as u32);
        versions.push(Version { to_old: vec![0], to_new: [(0, 0)].iter().cloned().collect(), ver });
      }
    } else {
      for (j, s) in sections {
        let v = s.parse().unwrap();
        // This versions list doesn't contain latest, so we have to subtract one.
        versions[j - 1].to_old.push(v);
        versions[j - 1].to_new.insert(v, i as u32);
      }
    }
  }

  // In this list, versions[0] is the latest version. The way the BlockVersion
  // enum is built, this is backwards. So we reverse it here, so that the types
  // using this list can just call BlockVersion::to_index().
  versions.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_convert() {
    let conv = TypeConverter::new();

    // This line makes it easy to test each version
    // 15743,11268,11252,8595,4080,4080,4080,4080,0

    assert_eq!(conv.to_old(15743, BlockVersion::V1_16), 15743);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_15), 11268);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_14), 11252);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_13), 8595);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_12), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_11), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_10), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_9), 4080);
    assert_eq!(conv.to_old(15743, BlockVersion::V1_8), 0);

    // Used to show debug output.
    // assert!(false);
  }
}
