# Threepen
Like bullpen, but with three bulls, two bulls, one bull, zero bulls!

## Strategy
Create the bull pen hint maker: For each region, see where you can and cannot place bulls. (Make dots)

We call optimizations any deductions we make that reduce the region size (where the bull cannot be).

Optimizations:
* Single Pen (only one space is available)
* One Direction (cell is all vertical)
* Pen Overlap (there exists a cell that invalidates the entire region)
* Overcounting (if k rows contains cells from exactly K regions, those regions' bulls must be in those rows/cols, and we can reduce the rest of the region)

Future optimizations:
* Improve Pen Overlap to include cells greater than 3
* Add undercounting (if needed)

## Generating Board
* Randomly creating regions, and seeing if it contains bulls is faster than generating bulls and then creating regions
* Use A* with number of solutions as heuristic
    * Give up after several tries to avoid bottlenecks

New heuristics to test:
* Create preset regions (4x4 etc) to lower search space


Useful: https://kris.pengy.ca/starbattle
Game: https://starbattle.puzzlebaron.com/init.php

## Algorithm X
Very fast solver [wikipedia](https://en.wikipedia.org/wiki/Knuth%27s_Algorithm_X)