# Voxel Water 
A real-time cellular automaton simulation of voxel fluids. Includes an efficient voxel renderer.

[ITCH DEMO](https://hiperslug.itch.io/magic),
[VIDEO DEMO](https://youtu.be/WJV30a9Xn5w)

## Stack
- Bevy
- Wgpu through bevy (Rust implementation of WebGPU standard)
- Rust

## Features
- Simulation
  - Liquid voxels
  - Order independent simulation
  - Efficient collision handling
- Meshing
  - Binary greedy meshing
  - Incremental meshing
- Rendering
  - Instanced quads
  - Texture indexing

### Collision
When multiple sources target the same destination, a noise based order independent priority resolves conflicts without collecting all sources.

### Incremental Meshing
Only the planes affected by an edit need to be remeshed.

## Limitations
- Single Chunk

## Roadmap
- Multi-chunk support for infinite worlds
  - Parallelism. Current implementation was built around this assumption.
  - Conditional simulation, loading, and rendering to reduce unnecessary load.
  - Streaming and generation. 
  - Extend the rendering pipeline with sub-allocated GPU buffers per chunk to reduce fragmentation and resource management.
