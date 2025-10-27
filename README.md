# Voxel Water 
A cellular based water simulation in a voxel grid. It's governed by simple gravity rules that make the water spread out.

Btw I'm using [Bevy](https://bevy.org/) for basically everything else.

[VIDEO](https://youtu.be/Sgi_PgaPVHo) demo. [COOL](https://hiperslug.itch.io/magic) demo

# Update #3
This is the continuation of my voxel water project. This week I did two major things:
1. Reworked the movement system. I changed the setup completely to make it WAY more readable as well as fix a litney of small problems. 
2. Added incremental meshing. I setup the meshing algorithm to be able to modify a mesh *in place* so every time I move a cell I only remesh the 12 faces that it directly influences instead of *all* of them.

# Future
I setup a very primitive rendering pipeline last week. I really should've worked on that but it *scares* me.

I also have to eventually move to a multi-chunk simulation. Which means suddenly I have a mix of dense and sparse data that I'm trying to handle, preferabbly in parrallel and/or in background threads.

# Gambling
I set a high personal bet on thursday then got **sick** on friday...

# Theme 
I got a freeby. This is literally water *falling*.
