# Voxel Water 
A cellular based water simulation in a voxel grid. It's governed by simple gravity rules that make the water spread out.

I'm using [Bevy](https://bevy.org/) for basically everything else.

[VIDEO](https://youtu.be/TMTr67NGoU0) demo. 
[ITCH](https://hiperslug.itch.io/magic) demo.

# Update #4
This is the continuation of my voxel water project. This week I mostly worked on rendering.

I added support for textures and have half-implimented support for transparent textures. I used this to add some spooky textures.

I also fixed the lighting (it was broken and I was just faking it).

# Future
I need to finish transparent textures. I also have a lot of work i *could* do on the rendering pipeline but I'm not sure it's worth it for this project.

Furthermore expanding this beyond a single chunk would be great.

I also think I want to, yet again, rework the chunk because it's been slowly changing as more requirements are put on it and I think I can jump the gun.

# Issues
Apparently people have had issues 
1. Crashing
2. Performance

This is probably because I'm messing around with custom stuff that isn't very cross platform. Unfortunately I haven't really looked into it yet.
