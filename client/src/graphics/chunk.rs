use common::chunk::Chunk;

/// A chunk with a mesh. This acts the same as a normal mesh, but will lazily
/// update a mesh any time it needs to be rendered.
pub struct MeshChunk {
  chunk: Chunk,
}

impl MeshChunk {
  pub fn new(chunk: Chunk) -> Self {
    MeshChunk { chunk }
  }
}