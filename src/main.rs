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

    flounder.uci_loop();
}
