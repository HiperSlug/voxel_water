# Voxel Water 
A cellular based water simulation in a voxel grid. It's governed by simple gravity rules that make the water spread out.

I'm using [Bevy](https://bevy.org/) for basically everything else.

[VIDEO](https://youtu.be/WJV30a9Xn5w) \
[ITCH](https://hiperslug.itch.io/magic)

I was finally to be rid of this project and then the theme was *grid*...

# Update #5
This is the continuation of my voxel water project.

I changed the bitmasks to be a grouped as an array of structures. It's more readable and should make writes faster (but iteration slower).

I also switched the simulation to two step. 1. Collect moves 2. Apply moves. I did this because previous changes forced me to setup the two-step infrastructure however I was still relying on a double buffered approach so I decided just to swap it completely.

I also completely switched up the data stored in a voxel. I had been storing a hardcoded enum. I've now set it up so that I'm now storing indices into a `Vec` of `Block` which encodes the actual data. It's still hardcoded just with a more flexible and realistic format. I also used this new format to encode specific texture patterns (mapping `Face => texture`) in each block.

I also tried to finish transparency but it just wasn't working and I didn't spend the time to figure it out.

I also made a rendering change that *might* fix the crashing. Idk though as I'm not experiencing the bug myself.

# Issues
People have had issues 
1. Crashing
2. Performance
