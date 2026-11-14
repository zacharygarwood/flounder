mod bitboard;
mod board;
mod correction;
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

    if args.get(1).map(|a| a.as_str()) == Some("tune") {
        // flounder tune <data> <epochs> <all|positional> <out>
        let scope = match (args.len(), args.get(4).map(|a| a.as_str())) {
            (6, Some("all")) => tune::Scope::All,
            (6, Some("positional")) => tune::Scope::Positional,
            _ => {
                eprintln!("usage: flounder tune <data> <epochs> <all|positional> <out>");
                std::process::exit(2);
            }
        };
        let epochs = args[3].parse().expect("epochs must be a number");
        if let Err(error) = tune::tune(&args[2], epochs, scope, &args[5]) {
            eprintln!("tune failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    flounder.uci_loop();
}
