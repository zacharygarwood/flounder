use crate::board::Board;
use crate::eval::Evaluator;
use crate::history::PositionHistory;
use crate::killer_moves::KillerMoves;
use crate::move_gen::MoveGenerator;
use crate::moves::{Move, MoveType};
use crate::timer::SearchTimer;
use crate::transposition::{Bounds, TranspositionTable};
use crate::zobrist::ZobristTable;
use std::cmp::{max, min};
use std::time::Duration;

/// Negative infinity for alpha-beta bounds (avoiding overflow)
const NEGATIVE_INFINITY: i32 = (i16::MIN + 1) as i32;

/// Positive infinity for alpha-beta bounds
const INFINITY: i32 = -NEGATIVE_INFINITY;

/// Checkmate score (leaves room for mate distance)
const CHECKMATE_SCORE: i32 = i32::MAX - 1000;

/// Most Valuable Victim - Least Valuable Attacker scores for move ordering
/// Rows: victim piece (King, Queen, Rook, Bishop, Knight, Pawn)
/// Columns: attacker piece (King, Queen, Rook, Bishop, Knight, Pawn)
pub const MVV_LVA_SCORES: [[i8; 6]; 6] = [
    [0, 0, 0, 0, 0, 0], // King capture should never happen
    [50, 51, 52, 53, 54, 55],
    [40, 41, 42, 43, 44, 45],
    [30, 31, 32, 33, 34, 35],
    [20, 21, 22, 23, 24, 25],
    [10, 11, 12, 13, 14, 15],
];

/// The main chess position searcher.
pub struct Searcher {
    move_generator: MoveGenerator,
    evaluator: Evaluator,
    zobrist: ZobristTable,
    transposition_table: TranspositionTable,
    killer_moves: KillerMoves,
    timer: SearchTimer,
    history: PositionHistory,
}

impl Searcher {
    /// Creates a new searcher with all components initialized
    pub fn new() -> Self {
        Self {
            move_generator: MoveGenerator::new(),
            evaluator: Evaluator::new(),
            zobrist: ZobristTable::new(),
            transposition_table: TranspositionTable::new(),
            killer_moves: KillerMoves::new(),
            timer: SearchTimer::new(),
            history: PositionHistory::new(),
        }
    }

    /// Finds the best move in the current position.
    ///
    /// Uses iterative deepening by searching depth 1, then 2, then 3, etc.
    /// This helps with move ordering since deeper searches can use results
    /// from shallower searches.
    ///
    /// # Arguments
    /// * `board` - The current position
    /// * `max_depth` - Maximum search depth in half moves
    /// * `time_limit` - Optional time limit for search
    ///
    /// # Returns
    /// Tuple of (evaluation score, best move)
    pub fn find_best_move(
        &mut self,
        board: &Board,
        max_depth: u8,
        time_limit: Option<Duration>,
    ) -> (i32, Option<Move>) {
        self.timer.start(time_limit);

        let mut best_score = NEGATIVE_INFINITY;
        let mut best_move = None;

        for current_depth in 1..=max_depth {
            if self.timer.should_stop() {
                break;
            }

            let result = self.search_position(board, current_depth);

            // Only update if search completed
            if !self.timer.should_stop() {
                best_score = result.score;
                best_move = result.best_move;

                self.cache_search_result(board, &result, current_depth);
                self.timer
                    .print_info(current_depth, result.score, result.best_move);
            }
        }

        (best_score, best_move)
    }

    /// Searches a position to a given depth using negamax with alpha-beta.
    fn search_position(&mut self, board: &Board, depth: u8) -> SearchResult {
        self.history.push(self.zobrist.hash(board));

        let result = self.negamax(
            board,
            depth,
            0,
            NEGATIVE_INFINITY,
            INFINITY,
            SearchContext::new(),
        );

        self.history.pop();
        result
    }

    /// Negamax search with alpha-beta pruning.
    ///
    /// Negamax is a variant of minimax where we always maximize from the
    /// current player's perspective.
    ///
    /// # Alpha-Beta Pruning
    /// - `alpha`: Best score we can guarantee (lower bound)
    /// - `beta`: Best score opponent can guarantee (upper bound)
    ///
    /// # Arguments
    /// * `board`: - Position to search
    /// * `depth` - Remaining depth to search
    /// * `ply` - Current ply from root
    /// * `alpha` - Best score for us so far
    /// * `beta` - Best score for opponent so far
    /// * `context` - Search context
    fn negamax(
        &mut self,
        board: &Board,
        depth: u8,
        ply: u8,
        mut alpha: i32,
        beta: i32,
        mut context: SearchContext,
    ) -> SearchResult {
        self.timer.increment_nodes();
        let original_alpha = alpha;

        if ply > 0 && self.is_draw_by_repetition(board) {
            return SearchResult::new(0, None);
        }

        // Check if we've already seen this position
        if let Some(cached_result) =
            self.probe_transposition_table(board, depth, alpha, beta, &mut context)
        {
            return cached_result;
        }

        // Quiescence search checks, captures, and promotions
        if depth == 0 {
            let score = self.search_until_quiet(board, alpha, beta);
            return SearchResult::new(score, None);
        }

        // Generate and order moves (best moves first for better pruning)
        let mut moves = self.move_generator.generate_moves(board);

        // Check for checkmate/stalemate
        if moves.is_empty() {
            return self.handle_terminal_position(board, depth);
        }

        self.order_moves(board, &mut moves, context.tt_best_move, ply);

        let mut best_result = SearchResult::worst();
        for current_move in moves {
            if self.timer.should_stop() {
                break;
            }

            let next_position = board.clone_with_move(&current_move);

            // Recursively search, flip the sign because we're switching sides
            let score = -self
                .negamax(
                    &next_position,
                    depth - 1,
                    ply + 1,
                    -beta,
                    -alpha,
                    SearchContext::new(),
                )
                .score;

            if score > best_result.score {
                best_result.score = score;
                best_result.best_move = Some(current_move);
            }

            alpha = max(alpha, score);
            if alpha >= beta {
                if current_move.move_type == MoveType::Quiet {
                    self.killer_moves.store(current_move, ply);
                }
                break;
            }
        }

        let bound = self.determine_bound(best_result.score, original_alpha, beta);
        self.store_in_transposition_table(board, &best_result, depth, bound);

        best_result
    }

    /// Searches until position is "quiet" (no captures, checks, or promotions)
    ///
    /// This prevents the "horizon effect" where the engine stops searching right
    /// before a capture sequence, leading to bad evaluations.
    fn search_until_quiet(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        self.timer.increment_nodes();
        let currently_in_check = self.move_generator.is_in_check(board);

        let mut moves = if currently_in_check {
            self.move_generator.generate_moves(board)
        } else {
            self.move_generator.generate_quiescence_moves(board)
        };

        self.order_captures(&mut moves, board);

        // Checkmate detection
        if moves.is_empty() && currently_in_check {
            return -CHECKMATE_SCORE;
        }

        let stand_pat = self.evaluator.evaluate(board);
        if stand_pat >= beta {
            return beta;
        }

        alpha = max(alpha, stand_pat);

        for mv in moves {
            if self.timer.should_stop() {
                break;
            }

            let next_position = board.clone_with_move(&mv);
            let score = -self.search_until_quiet(&next_position, -beta, -alpha);

            if score >= beta {
                return beta;
            }

            alpha = max(alpha, score);
        }

        alpha
    }

    fn is_draw_by_repetition(&self, board: &Board) -> bool {
        let current_hash = self.zobrist.hash(board);
        self.history.is_repetition(current_hash)
    }

    /// Checks if we've already searched this position
    fn probe_transposition_table(
        &self,
        board: &Board,
        depth: u8,
        mut alpha: i32,
        mut beta: i32,
        context: &mut SearchContext,
    ) -> Option<SearchResult> {
        let position_hash = self.zobrist.hash(board);
        let entry = self.transposition_table.retrieve(position_hash)?;

        // Store TT move for move ordering even if depth is insufficient
        context.tt_best_move = entry.best_move;

        // Only use entry if it was searched to sufficient depth
        if entry.depth < depth {
            return None;
        }

        match entry.bounds {
            Bounds::Exact => {
                return Some(SearchResult::new(entry.eval, entry.best_move));
            }
            Bounds::Lower => {
                alpha = max(alpha, entry.eval);
            }
            Bounds::Upper => {
                beta = min(beta, entry.eval);
            }
        }

        if alpha >= beta {
            return Some(SearchResult::new(entry.eval, entry.best_move));
        }

        // Can't use this entry
        None
    }

    /// Stores a search result in the transposition table.
    fn store_in_transposition_table(
        &mut self,
        board: &Board,
        result: &SearchResult,
        depth: u8,
        bound: Bounds,
    ) {
        let position_hash = self.zobrist.hash(board);
        self.transposition_table
            .store(position_hash, result.score, result.best_move, depth, bound);
    }

    /// Caches the result from iterative deepening for move ordering.
    fn cache_search_result(&mut self, board: &Board, result: &SearchResult, depth: u8) {
        self.store_in_transposition_table(board, result, depth, Bounds::Exact);
    }

    /// Determines the bound type for a transposition table entry.
    fn determine_bound(&self, score: i32, original_alpha: i32, beta: i32) -> Bounds {
        if score <= original_alpha {
            Bounds::Upper
        } else if score >= beta {
            Bounds::Lower
        } else {
            Bounds::Exact
        }
    }

    /// Handles terminal positions
    fn handle_terminal_position(&self, board: &Board, depth: u8) -> SearchResult {
        if self.move_generator.is_in_check(board) {
            // Prefer shorter mates
            let mate_score = -CHECKMATE_SCORE + depth as i32;
            SearchResult::checkmate(mate_score)
        } else {
            SearchResult::stalemate()
        }
    }

    /// Orders moves for better alpha-beta pruning.
    ///
    /// Priority:
    /// 1. Transposition table move
    /// 2. Captures (MVV-LVA)
    /// 3. Killer moves
    /// 4. Promotions
    /// 5. Other moves
    fn order_moves(&self, board: &Board, moves: &mut [Move], tt_move: Option<Move>, ply: u8) {
        moves.sort_by_cached_key(|mv| {
            if let Some(best_move) = tt_move {
                if *mv == best_move {
                    return i16::MIN;
                }
            }

            if mv.move_type == MoveType::Capture || mv.move_type == MoveType::EnPassant {
                if let Some(score) = self.calculate_capture_score(board, mv) {
                    return -(score as i16) - 1000;
                }
            }

            if self.killer_moves.is_killer(mv, ply) {
                return -500;
            }

            if mv.move_type == MoveType::Promotion {
                return -400;
            }

            // Quiet moves last
            0
        });
    }

    /// Orders captures using MVV-LVA
    fn order_captures(&self, moves: &mut [Move], board: &Board) {
        moves.sort_by_cached_key(|mv| {
            if mv.move_type == MoveType::EnPassant {
                return -10;
            }

            self.calculate_capture_score(board, mv)
                .map(|score| -score)
                .unwrap_or(0)
        });
    }

    /// Calculates the capture score for MVV-LVA ordering
    fn calculate_capture_score(&self, board: &Board, mv: &Move) -> Option<i8> {
        let attacker = board.get_piece_at(mv.from)?;
        let victim = board.get_piece_at(mv.to)?;

        Some(MVV_LVA_SCORES[victim.index()][attacker.index()])
    }

    /// Updates position history (for repetition detection)
    #[allow(dead_code)]
    fn push_position(&mut self, board: &Board) {
        self.history.push(self.zobrist.hash(board));
    }
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a search operation
#[derive(Debug, Clone, Copy)]
struct SearchResult {
    score: i32,
    best_move: Option<Move>,
}

impl SearchResult {
    fn new(score: i32, best_move: Option<Move>) -> Self {
        Self { score, best_move }
    }

    fn worst() -> Self {
        Self {
            score: NEGATIVE_INFINITY,
            best_move: None,
        }
    }

    fn checkmate(checkmate_score: i32) -> Self {
        Self {
            score: checkmate_score,
            best_move: None,
        }
    }

    fn stalemate() -> Self {
        Self {
            score: 0,
            best_move: None,
        }
    }
}

/// Context for search
#[derive(Debug, Clone, Copy)]
struct SearchContext {
    tt_best_move: Option<Move>,
}

impl SearchContext {
    fn new() -> Self {
        Self { tt_best_move: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_DEPTH: u8 = 6;

    /// Helper function to test if engine finds the correct move in positions.
    fn assert_finds_move(fen: &str, expected_move: &str) {
        let board = Board::new(fen);
        let mut searcher = Searcher::new();
        let (score, best_move) = searcher.find_best_move(&board, SEARCH_DEPTH, None);

        assert!(best_move.is_some(), "Engine should find a move");
        assert_eq!(
            best_move.unwrap().to_algebraic(),
            expected_move,
            "Wrong move found (score: {})",
            score
        );
    }

    #[test]
    fn finds_back_rank_mate() {
        assert_finds_move("4k3/5p2/8/6B1/8/8/8/3R2K1 w - - 0 1", "d1d8");
    }

    #[test]
    fn finds_queen_sacrifice_mate() {
        assert_finds_move(
            "rn1r2k1/ppp2ppp/3q1n2/4b1B1/4P1b1/1BP1Q3/PP3PPP/RN2K1NR b KQ - 0 1",
            "d6d1",
        );
    }

    #[test]
    fn finds_smothered_mate_pattern() {
        assert_finds_move("6k1/6P1/5K1R/8/8/8/8/8 w - - 0 1", "h6h8");
    }

    // Positions found here:
    // https://lichess.org/practice/checkmates/checkmate-patterns-iii/
    #[test]
    fn opera_mate_1() {
        assert_finds_move("4k3/5p2/8/6B1/8/8/8/3R2K1 w - - 0 1", "d1d8");
    }

    #[test]
    fn opera_mate_2() {
        assert_finds_move(
            "rn1r2k1/ppp2ppp/3q1n2/4b1B1/4P1b1/1BP1Q3/PP3PPP/RN2K1NR b KQ - 0 1",
            "d6d1",
        );
    }

    #[test]
    fn opera_mate_3() {
        assert_finds_move(
            "rn3rk1/p5pp/2p5/3Ppb2/2q5/1Q6/PPPB2PP/R3K1NR b KQ - 0 1",
            "c4f1",
        );
    }

    #[test]
    fn anderssens_mate_1() {
        assert_finds_move("6k1/6P1/5K1R/8/8/8/8/8 w - - 0 1", "h6h8");
    }

    #[test]
    fn anderssens_mate_2() {
        assert_finds_move(
            "1k2r3/pP3pp1/8/3P1B1p/5q2/N1P2b2/PP3Pp1/R5K1 b - - 0 1",
            "f4h4",
        );
    }

    #[test]
    fn anderssens_mate_3() {
        assert_finds_move(
            "2r1nrk1/p4p1p/1p2p1pQ/nPqbRN2/8/P2B4/1BP2PPP/3R2K1 w - - 0 1",
            "f5e7",
        );
    }

    #[test]
    fn dovetail_mate_1() {
        assert_finds_move("1r6/pk6/4Q3/3P4/8/8/8/6K1 w - - 0 1", "e6c6");
    }

    #[test]
    fn dovetail_mate_2() {
        assert_finds_move(
            "r1b1q1r1/ppp3kp/1bnp4/4p1B1/3PP3/2P2Q2/PP3PPP/RN3RK1 w - - 0 1",
            "f3f6",
        );
    }

    #[test]
    fn dovetail_mate_3() {
        assert_finds_move(
            "6k1/1p1b3p/2pp2p1/p7/2Pb2Pq/1P1PpK2/P1N3RP/1RQ5 b - - 0 1",
            "d7g4",
        );
    }

    #[test]
    fn dovetail_mate_4() {
        assert_finds_move("rR6/5k2/2p3q1/4Qpb1/2PB1Pb1/4P3/r5R1/6K1 w - - 0 1", "e5e8");
    }

    #[test]
    fn cozios_mate_1() {
        assert_finds_move("8/8/1Q6/8/6pk/5q2/8/6K1 w - - 0 1", "b6h6");
    }

    #[test]
    fn swallows_tail_mate_1() {
        assert_finds_move("3r1r2/4k3/R7/3Q4/8/8/8/6K1 w - - 0 1", "d5e6");
    }

    #[test]
    fn swallows_tail_mate_2() {
        assert_finds_move("8/8/2P5/3K1k2/2R3p1/2q5/8/8 b - - 0 1", "c3e5");
    }

    #[test]
    fn epaulette_mate_1() {
        assert_finds_move("3rkr2/8/5Q2/8/8/8/8/6K1 w - - 0 1", "f6e6");
    }

    #[test]
    fn epaulette_mate_2() {
        assert_finds_move(
            "1k1r4/pp1q1B1p/3bQp2/2p2r2/P6P/2BnP3/1P6/5RKR b - - 0 1",
            "d8g8",
        );
    }

    #[test]
    fn epaulette_mate_3() {
        assert_finds_move("5r2/pp3k2/5r2/q1p2Q2/3P4/6R1/PPP2PP1/1K6 w - - 0 1", "f5d7");
    }

    #[test]
    fn pawn_mate_1() {
        assert_finds_move("8/7R/1pkp4/2p5/1PP5/8/8/6K1 w - - 0 1", "b4b5");
    }

    #[test]
    fn pawn_mate_2() {
        assert_finds_move(
            "r1b3nr/ppp3qp/1bnpk3/4p1BQ/3PP3/2P5/PP3PPP/RN3RK1 w - - 0 11",
            "h5e8",
        );
    }

    #[test]
    fn test_repetition_detection() {
        let mut searcher = Searcher::new();
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        // Simulate three-fold repetition
        searcher.push_position(&board);
        searcher.push_position(&board);
        searcher.push_position(&board);

        assert!(searcher.is_draw_by_repetition(&board));
    }

    #[test]
    fn test_search_speed() {
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let mut searcher = Searcher::new();

        let start = std::time::Instant::now();
        searcher.find_best_move(&board, 4, None);
        let duration = start.elapsed();

        assert!(duration.as_secs() < 10, "Search too slow: {:?}", duration);
    }

    #[test]
    fn test_time_management() {
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let mut searcher = Searcher::new();

        let time_limit = Duration::from_millis(100);
        let start = std::time::Instant::now();
        searcher.find_best_move(&board, 10, Some(time_limit));
        let duration = start.elapsed();

        // Should respect time limit
        assert!(
            duration.as_millis() <= time_limit.as_millis() + 50,
            "Exceeded time limit: {:?} vs {:?}",
            duration,
            time_limit
        );
    }
}
