use bullpen::BullpenSolver;

/// Render the board, marking bull cells with `[n]` around the region label.
fn print_board(regions: &[Vec<usize>], bulls: &[(usize, usize)]) {
    for (r, row) in regions.iter().enumerate() {
        let line: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(c, &region)| {
                if bulls.contains(&(r, c)) {
                    format!("* ")
                } else {
                    format!("{region} ")
                }
            })
            .collect();
        println!("{}", line.join(""));
    }
}

fn main() {
    let regions = vec![
        vec![5, 1, 4, 2, 2, 2, 2, 2],
        vec![5, 1, 4, 2, 2, 2, 2, 2],
        vec![5, 1, 2, 2, 2, 2, 0, 2],
        vec![5, 5, 5, 5, 5, 5, 0, 2],
        vec![5, 5, 5, 5, 5, 5, 2, 2],
        vec![5, 5, 5, 5, 5, 6, 2, 2],
        vec![5, 5, 5, 5, 5, 2, 2, 7],
        vec![5, 5, 3, 3, 2, 2, 7, 7],
    ];

    println!("Board:");
    print_board(&regions, &[]);

    let solutions = BullpenSolver::new(&regions).solve();
    println!(
        "\n{} solution{} found",
        solutions.len(),
        if solutions.len() == 1 { "" } else { "s" }
    );

    for (i, bulls) in solutions.iter().enumerate() {
        let coords: Vec<String> = bulls.iter().map(|(r, c)| format!("({r}, {c})")).collect();
        println!("\nSolution {}: bulls at {}", i + 1, coords.join(", "));
        print_board(&regions, bulls);
    }

    // demo the generator: fresh random 8x8 puzzle each run
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as u64;
    let generated = bullpen::generate(27, seed);
    println!("\nGenerated board (seed {seed}):");
    print_board(&generated, &[]);

    let bulls = &BullpenSolver::new(&generated).solve()[0];
    println!("\nIts unique solution:");
    print_board(&generated, bulls);
}
