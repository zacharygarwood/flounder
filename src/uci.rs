use crate::board::Board;
use crate::search::Searcher;
use crate::move_gen::MoveGenerator;

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
    pub fn uci_loop(&mut self) {
        loop {
            let mut command = String::new();
            if let Ok(_) = std::io::stdin().read_line(&mut command) {
                command = command.trim().to_string();
                self.handle_command(&command);
            }
        }
    }

    fn handle_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split(' ').collect();

        match parts[0] {
            "uci" => {
                self.handle_uci_command();
            }
            "isready" => {
                self.handle_isready_command();
            }
            "ucinewgame" => {
                self.handle_ucinewgame_command();
            }
            "position" => {
                self.handle_position_command(&parts);
            }
            "go" => {
                self.handle_go_command();
            }
            "quit" => {
                std::process::exit(0);
            }
            _ => {
                // Handle unknown command
            }
        }
    }

    fn handle_uci_command(&self) {
        println!("id name Flounder");
        println!("id author Zachary Garwood");
        println!("uciok");
    }

    fn handle_isready_command(&mut self) {
        println!("readyok");
    }

    fn handle_ucinewgame_command(&mut self) {
        self.board = Board::default();
    }

    fn handle_position_command(&mut self, parts: &[&str]) {
        let position_type = parts[1];
        if position_type == "startpos" {
            self.board = Board::default();
            
            if parts.len() > 2 && parts[2] == "moves" {
                self.make_moves(&parts[3..]);
            }
        } else if position_type == "fen" {
            let fen = parts[2..8].join(" ");
            self.board = Board::new(&fen);
    
            if parts.len() > 8 && parts[8] == "moves" {
                self.make_moves(&parts[9..]);
            }
        }
    }

    fn handle_go_command(&mut self) {
        let (_, best_move) = self.searcher.best_move(&self.board, 6);
        println!("bestmove {}", best_move.unwrap().to_algebraic());
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
