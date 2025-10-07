# Threepen
Like bullpen, but with three bulls, two bulls, one bull, zero bulls!

## Heuristics
* Try generating with bias: make some spaces larger
    * Benchmark is average solution space **(lower better)**
* Generate spaces first, and see if it is a valid bullpen
    * Percentage of boards that have no solution
    * Benchmark is average solution space
* Fix board targeting large boxes vs small boxes
    * Benchmark is # of tries until 1 solution
* Try regenerating until # of solutions is less than 20
    * Percentage of boards with solutions > 20 and < 20
    * Benchmark is # of tries until 1 solution