#roguelike-rs
### aka. I followed a tutorial to make a roguelike and want to see where I can go with this
### Backstory / Goal
I'm a huge fan of roguelikes and have always wanted to make my own, this is me doing that. I plan to slowly develop this into a fully featured roguelike but for now have based the majority of it off of [this tutorial](https://tomassedovic.github.io/roguelike-tutorial/). As of writing I've nearly completed the tutorial with my own alterations and will be moving onto refactoring some of the code and implementing features. If I get far enough into this project I plan to move it to a new repo under a new name and actually treat it as a proper game.
### Future Plan
This is the order in which I plan to start proceeding with development after finishing the tutorial
1. Refactoring the code into multiple separate modules in order to improve readability. As of now the code is over 1000 lines long and is already a massive pain in the ass to find things in with out searching. splitting things out will allow me to better implement new features and update existing ones.
2. New graphics! I want to try and use a tileset that will allow for both a good font and some actual sprites in order to spruce up the game and make it look purdy. A UI rewrite will also definitely be included in here I intend to switch from a bottom based ui to a side based ui. I also would love to figure out some way to implement some ASCII animations to make shit really nice.
3. Map generation algorithm rewrite. As of now the provided example map generation can create some really odd layouts and is fairly bland. With some time I plan to make it more varied and try and make it much better.
4. AI Rewrite + Multiple new AI. Thanks to the given systems adding multiple types of AI is actually something really easily doable as well as something that is much needed because as of now monsters will not follow the player down corridors and get stuck on each other. on top of this they do this really fucking stupid thing where they can attack the player from a diagonal which the player is incapable of countering.
5. Critters / lore / items / flavor text. self explanatory I think. I just want to flesh it out a lot more
6. Classes. I intend to implement a few classes into the game to add more replayabilty and varied play style. This might be moved further down the list due to the fact that it might be a lot to implement at once and I honestly have no idea how I'm going to go about handling it.

### Using my shitty code
idfk clone repo then `cargo build` or some shit fam. 