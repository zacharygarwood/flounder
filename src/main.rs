mod bitboard;
mod board;
mod eval;
mod fen;
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
    let mut flounder = Flounder::new();
    flounder.uci_loop();
}
