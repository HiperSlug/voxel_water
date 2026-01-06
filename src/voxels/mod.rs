pub mod block;
pub mod chunk;
pub mod space;

use bevy::prelude::*;
use chunk::Chunk;
use dashmap::DashMap;

use crate::render::{ChunkMesh, ChunkRemesh};

#[derive(Default, Component)]
pub struct SparseChunks {
    map: DashMap<IVec3, Chunk>,
}

#[derive(Default, Component)]
pub struct SparseChunkMeshes {
    map: DashMap<IVec3, ChunkMesh>,
}

#[derive(Default, Component)]
pub struct SparseChunkRemeshes {
    map: DashMap<IVec3, ChunkRemesh>,
}

// impl SparseChunks {
//     pub fn fill(&self, range: Range<VoxelPos>, voxel: Option<BlockIndex>) -> () {
//         for z in range.start.z..=range.end.z {
//             for y in range.start.y..=range.end.y {
//                 for x in range.start.x..=range.end.x {
//                     let position = VoxelPos::new(x, y, z);
//                     for (big, small) in position.all_chunk_pairs() {
//                         self.map.entry(big).or_default().set(small, voxel.unwrap());
//                     }
//                 }
//             }
//         }
//     }
// }
