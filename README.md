# engiffen

Generates gifs from image files.

## Minor work to do

* CLI
* Specifying framerate

## Major work to do

* Incremental frame processing

  Accept a stream of frames from `stdin` to process individually as they arrive. Put off sorting the final palette and compiling the gif until finished.

* Handle sources with more than 256 colors total

  Naiive implementation is to rank colors based on their frequency in all frames, then adding the
  256 most frequently used colors to the palette. After palette is filled, for each new color find
  the closest matching color in the palette and use it instead.

  This is what we currently do, and it has serious limitations on true-color gifs, because it does
  not determine which colors are important and which are insignificant.

## Anything else?

![shrug](test/shrug.gif)
