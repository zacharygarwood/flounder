use crate::board::Board;
use crate::move_gen::MoveGenerator;
use crate::pieces::Color;
use crate::search::Searcher;
use std::time::Duration;

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
                            time_limit = Some(Duration::from_millis(ms));
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "wtime" | "btime" | "winc" | "binc" => {
                    time_limit = self.calculate_move_time(parts, i);
                    i += 8;
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

    /// Calculates how much time to use for this move
    fn calculate_move_time(&self, parts: &[&str], start_idx: usize) -> Option<Duration> {
        let color = self.board.active_color();

        let mut wtime = 0u64;
        let mut btime = 0u64;
        let mut winc = 0u64;
        let mut binc = 0u64;

        let mut i = start_idx;
        while i < parts.len() {
            match parts[i] {
                "wtime" => {
                    if i + 1 < parts.len() {
                        wtime = parts[i + 1].parse().unwrap_or(0);
                    }
                    i += 2;
                }
                "btime" => {
                    if i + 1 < parts.len() {
                        btime = parts[i + 1].parse().unwrap_or(0);
                    }
                    i += 2;
                }
                "winc" => {
                    if i + 1 < parts.len() {
                        winc = parts[i + 1].parse().unwrap_or(0);
                    }
                    i += 2;
                }
                "binc" => {
                    if i + 1 < parts.len() {
                        binc = parts[i + 1].parse().unwrap_or(0);
                    }
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }

        let (time_left, increment) = match color {
            Color::White => (wtime, winc),
            Color::Black => (btime, binc),
        };

        let reserve = 5_000; // Try to always keep 5 seconds
        let available = time_left.saturating_sub(reserve);
        let base_time = available / 25;
        let allocated = base_time + increment;

        Some(Duration::from_millis(allocated))
    }

    fn make_moves(&mut self, move_strs: &[&str]) {
        let move_gen = MoveGenerator::new();
        for mv_str in move_strs.iter() {
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
}
