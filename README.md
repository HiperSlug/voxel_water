# Voxel Water
A real-time voxel simulation of water physics. 

At its core each cell just tries to move into each of the 9 cells below it. If that fails it slides away from adjacent cells.

I setup a decent cpu (not gpu b/c real-time) simulation based on some techniques I learned from [binary-greedy-meshing](https://github.com/Inspirateur/binary-greedy-meshing) as well as my own thoughts on how to best handle collisions in a consistent (if not deterministic) way. I also included an implementation of binary-greedy-meshing for ridiculously fast meshing thats far more readable than the original (in my opinion). 

I also learned how voxel raytracing works.

# Future
Move priority. Because of iteration order certain cells have move priority resulting in non-symetrical simulation. I have a solution that would resolve this but it requires some rewriting.

Meshing. The chunk is changing every frame. I think I can impliment iterative (modify an old mesh instead of rebuilding) meshing using the same algorithms I do now.

Rendering. Currently I'm instancing Quad meshes using bevys mesh instancing. A custom rendering pipeline would be faster and allow me to cull about twice as many quads. Also texturing, transparency, and non-full blocks.

# Magic
There are wands! YAYYYY!
