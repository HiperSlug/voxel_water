# Voxel Water 
A cellular based water simulation in a voxel grid. It's governed by simple gravity and kicking (pushing away from eachother) rules that allow the water to spread out.

# Update
This is the continuation of my voxel water project. This week I setup a custom rendering pipeline for instanced quads based on this [video](https://www.youtube.com/watch?v=40JzyaOYJeY) with adaptations and concessions based on what I'm doing. It currently acts the same as the previous renderer however it's true utility comes in the ability to manually control data flow to allow better culling as well as cheaper per-instance data. 

Additionally I fixed the collision bias where certain cells would get priority if multiple tried to go to the same cell. My inital solution didn't work and so I replaced it with a suboptimal but random solution. I have an idea on something that might be faster my having reversable moves.

# Theme 
This project is loosly liked to the theme "Signals". For example voxel changes propagate signals through chains of liquid like physics. Additionally this week quad data needs to be constantly streamed (signaled) to the gpu to render updated voxels.
