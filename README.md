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
* Solve the board, capped at 2 solutions
    * 0 solutions: reroll
    * 1 solution: done
    * 2 solutions: repair
* Targeted repair: (Keep and Kill)
    * Find a bull cell of kill that is not in keep, and move that cell into an adjacent region
    (heuristic: num solutions should trend downwards)


Useful: https://kris.pengy.ca/starbattle
Game: https://starbattle.puzzlebaron.com/init.php

## Algorithm X
Very fast solver [wikipedia](https://en.wikipedia.org/wiki/Knuth%27s_Algorithm_X)