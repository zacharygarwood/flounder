use crate::board::Board;
use crate::move_gen::MoveGenerator;
use crate::pieces::Color;
use crate::search::Searcher;
use crate::timer::TimeLimits;
use std::time::Duration;

/// A fixed suite of positions for the `bench` command, spanning openings,
/// middlegames, tactical positions, and endgames.
const BENCH_POSITIONS: [&str; 12] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    "2rq1rk1/pp1bppbp/2np1np1/8/3NP3/2N1BP2/PPPQ2PP/2KR1B1R w - - 0 1",
    "r1bqkb1r/pp3ppp/2n1pn2/2pp4/3P1B2/2P1PN2/PP1N1PPP/R2QKB1R w KQkq - 0 1",
    "8/8/8/2k5/2pP4/8/B7/4K3 b - d3 0 1",
    "8/5k2/3p4/1p1Pp2p/pP2Pp1P/P4P1K/8/8 b - - 0 1",
    "5rk1/1ppb3p/p1pb4/6q1/3P1p1r/2P1R2P/PP1BQ1P1/5RKN w - - 0 1",
    "8/3k4/8/8/8/8/3K4/3R4 w - - 0 1",
];

/// Main UCI protocol handler
pub struct Flounder {
    board: Board,
    searcher: Searcher,
}

impl Flounder {
    pub fn new() -> Self {
        Self {
            board: Board::default(),
            searcher: Searcher::new(),
        }
    }

    /// Main UCI loop that reads and processes commands
    pub fn uci_loop(&mut self) {
        loop {
            let mut command = String::new();
            if std::io::stdin().read_line(&mut command).is_ok() {
                command = command.trim().to_string();
                if !command.is_empty() {
                    self.handle_command(&command);
                }
            }
        }
    }

    fn handle_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "uci" => self.handle_uci_command(),
            "isready" => self.handle_isready_command(),
            "ucinewgame" => self.handle_ucinewgame_command(),
            "position" => self.handle_position_command(&parts),
            "go" => self.handle_go_command(&parts),
            "bench" => self.bench(&parts),
            "quit" => std::process::exit(0),
            _ => {
                // Handle unknown command
            }
        }
    }

    /// Responds to UCI initialization
    fn handle_uci_command(&self) {
        println!("id name Flounder");
        println!("id author Zachary Garwood");
        println!("uciok");
    }

    /// Responds that the engine is ready
    fn handle_isready_command(&mut self) {
        println!("readyok");
    }

    /// Prepares a new game
    fn handle_ucinewgame_command(&mut self) {
        self.board = Board::default();
        self.searcher = Searcher::new();
    }

    /// Sets up the board position
    fn handle_position_command(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            return;
        }

        let position_type = parts[1];

        match position_type {
            "startpos" => {
                self.board = Board::default();
                self.searcher.reset_repetition();

                if let Some(moves_idx) = parts.iter().position(|&x| x == "moves") {
                    self.make_moves(&parts[moves_idx + 1..]);
                }
            }
            "fen" => {
                if parts.len() < 8 {
                    return;
                }

                let fen = parts[2..8].join(" ");
                self.board = Board::new(&fen);
                self.searcher.reset_repetition();

                if let Some(moves_idx) = parts.iter().position(|&x| x == "moves") {
                    self.make_moves(&parts[moves_idx + 1..]);
                }
            }
            _ => {}
        }
    }

    /// Starts the search with time controls
    fn handle_go_command(&mut self, parts: &[&str]) {
        let mut depth = 64; // High depth will get cut off by timer
        let mut time_limit = None;

        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "depth" => {
                    if i + 1 < parts.len() {
                        if let Ok(d) = parts[i + 1].parse::<u8>() {
                            depth = d.min(64);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "movetime" => {
                    if i + 1 < parts.len() {
                        if let Ok(ms) = parts[i + 1].parse::<u64>() {
                            time_limit = Some(TimeLimits::fixed(Duration::from_millis(ms)));
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "wtime" | "btime" | "winc" | "binc" | "movestogo" => {
                    // calculate_move_time scans all clock tokens itself, so only
                    // compute the budget once, from the first one seen.
                    if time_limit.is_none() {
                        time_limit = self.calculate_move_time(parts, i);
                    }
                    i += 1;
                }
                "infinite" => {
                    depth = 64;
                    time_limit = None;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        let (_, best_move) = self.searcher.find_best_move(&self.board, depth, time_limit);

        if let Some(mv) = best_move {
            println!("bestmove {}", mv.to_algebraic());
        } else {
            // No legal moves
            println!("bestmove 0000");
        }
    }

    /// Searches a fixed suite of positions to a fixed depth and reports the
    /// total node count and nodes per second. With deterministic zobrist keys
    /// the node count is reproducible, so a behavior-preserving optimization
    /// must leave it unchanged while raising nps.
    pub fn bench(&mut self, parts: &[&str]) {
        let depth: u8 = parts.get(1).and_then(|d| d.parse().ok()).unwrap_or(12);

        let mut searcher = Searcher::new();
        let mut total_nodes = 0u64;
        let start = std::time::Instant::now();

        for fen in BENCH_POSITIONS {
            let board = Board::new(fen);
            searcher.reset_repetition();
            searcher.find_best_move(&board, depth, None);
            total_nodes += searcher.last_search_nodes();
        }

        let elapsed = start.elapsed();
        let nps = (total_nodes as f64 / elapsed.as_secs_f64()) as u64;
        println!(
            "bench: depth {} nodes {} time {} ms nps {}",
            depth,
            total_nodes,
            elapsed.as_millis(),
            nps
        );
    }

    /// Calculates how much time to use for this move
    fn calculate_move_time(&self, parts: &[&str], start_idx: usize) -> Option<TimeLimits> {
        let color = self.board.active_color();

        let mut wtime = 0u64;
        let mut btime = 0u64;
        let mut winc = 0u64;
        let mut binc = 0u64;
        let mut movestogo = 0u64;

        let mut i = start_idx;
        while i + 1 < parts.len() {
            let value = parts[i + 1].parse().unwrap_or(0);
            match parts[i] {
                "wtime" => wtime = value,
                "btime" => btime = value,
                "winc" => winc = value,
                "binc" => binc = value,
                "movestogo" => movestogo = value,
                _ => {
                    i += 1;
                    continue;
                }
            }
            i += 2;
        }

        let (time_left, increment) = match color {
            Color::White => (wtime, winc),
            Color::Black => (btime, binc),
        };

        // Keep a small buffer so we never overstep the clock on transmission.
        let overhead = 30;
        let usable = time_left.saturating_sub(overhead);

        // Spread the remaining time over the moves still to play (a fixed
        // estimate when the GUI does not say), and add most of the increment.
        let moves_to_go = if movestogo > 0 { movestogo } else { 30 };
        let soft = (time_left / moves_to_go + increment * 3 / 4).min(usable);

        // Allow a single difficult move to borrow ahead, but never the whole
        // clock.
        let hard = (soft * 3).min(usable);

        Some(TimeLimits {
            soft: Duration::from_millis(soft),
            hard: Duration::from_millis(hard),
        })
    }

    fn make_moves(&mut self, move_strs: &[&str]) {
        let move_gen = MoveGenerator::new();
        for mv_str in move_strs.iter() {
            // Record the position before each move so the search can detect
            // threefold repetitions against the actual game line.
            self.searcher.record_game_position(&self.board);

            let moves = move_gen.generate_moves(&self.board);
            let mv = moves.iter().find(|m| m.to_algebraic() == *mv_str);
            self.board.make_move(mv.unwrap());
        }
    }
}

impl Default for Flounder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci_initialization() {
        Flounder::new();
        // No panics
    }

    #[test]
    fn test_position_parsing() {
        let mut flounder = Flounder::new();
        flounder.handle_command("position startpos");
        flounder.handle_command("position startpos moves e2e4 e7e5");
        // No panics
    }

    #[test]
    fn test_go_command() {
        let mut flounder = Flounder::new();
        flounder.handle_command("position startpos");
        flounder.handle_command("go depth 1");
        // No panics
    }

    fn budget(flounder: &Flounder, command: &str) -> TimeLimits {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let start = parts.iter().position(|p| *p == "wtime").unwrap();
        flounder.calculate_move_time(&parts, start).unwrap()
    }

    #[test]
    fn allocates_meaningful_time_at_fast_control() {
        // At 8s + 80ms the old flat 5s reserve collapsed to increment-only.
        let flounder = Flounder::new(); // White to move
        let limits = budget(&flounder, "go wtime 8000 btime 8000 winc 80 binc 80");
        assert_eq!(limits.soft, Duration::from_millis(8000 / 30 + 80 * 3 / 4));
        assert_eq!(limits.hard, limits.soft * 3);
        assert!(limits.soft > Duration::from_millis(80));
    }

    #[test]
    fn uses_side_to_move_clock() {
        let mut flounder = Flounder::new();
        flounder.handle_command("position startpos moves e2e4"); // Black to move
        let limits = budget(&flounder, "go wtime 8000 btime 4000 winc 80 binc 40");
        assert_eq!(limits.soft, Duration::from_millis(4000 / 30 + 40 * 3 / 4));
    }

    #[test]
    fn movestogo_spreads_remaining_time() {
        let flounder = Flounder::new();
        let limits = budget(&flounder, "go wtime 10000 btime 10000 movestogo 5");
        assert_eq!(limits.soft, Duration::from_millis(10000 / 5));
    }

    #[test]
    fn never_exceeds_available_time() {
        let flounder = Flounder::new();
        let limits = budget(&flounder, "go wtime 200 btime 200 winc 0 binc 0");
        // usable = 200 - 30 overhead.
        assert!(limits.hard <= Duration::from_millis(170));
        assert!(limits.soft <= limits.hard);
    }
}
