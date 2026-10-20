mod bitboard;
mod board;
mod eval;
mod fen;
mod history;
mod killer_moves;
mod lookup;
mod magic;
mod move_gen;
mod moves;
mod pieces;
mod repetition;
mod search;
mod square;
mod timer;
mod transposition;
mod tune;
mod uci;
mod util;
mod zobrist;

use uci::Flounder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut flounder = Flounder::new();

    if args.get(1).map(|a| a.as_str()) == Some("bench") {
        let bench_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
        flounder.bench(&bench_args);
        return;
    }

    // Texel tuning: `datagen <games> <seed> <depth> <out>` writes labeled
    // self-play positions; `tune <data> <epochs> <out>` fits the eval weights.
    match args.get(1).map(|a| a.as_str()) {
        Some("datagen") if args.len() == 6 => {
            let parse = |i: usize| args[i].parse::<u64>().expect("numeric argument");
            tune::datagen(parse(2) as usize, parse(3), parse(4) as u8, &args[5])
                .expect("failed to write datagen output");
            return;
        }
        Some("tune") if args.len() == 5 => {
            let epochs = args[3].parse().expect("numeric epoch count");
            tune::tune(&args[2], epochs, &args[4]).expect("tuning failed");
            return;
        }
        _ => {}
    }

    flounder.uci_loop();
}
