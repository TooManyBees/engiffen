# engiffen

Generates gifs from image sequences.

![source bitmap](tests/ball/ball01.bmp)
![engiffenned gif](tests/ball.gif)
![photoshopped gif](tests/ball_ps.gif)

_Source frame, generated gif, and a gif from Photoshop_

## Minor work to do

* CLI
* Specifying framerate

## Major work to do

* Incremental frame processing

  Accept a stream of frames from `stdin` to process individually as they arrive. Put off sorting the final palette and compiling the gif until finished.

## Anything else?

![shrug](tests/shrug.gif)
