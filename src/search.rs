use crate::board::Board;
use crate::eval::Evaluator;
use crate::history::HistoryTable;
use crate::killer_moves::KillerMoves;
use crate::move_gen::MoveGenerator;
use crate::moves::{Move, MoveType};
use crate::pieces::Piece;
use crate::repetition::RepetitionTable;
use crate::timer::{SearchTimer, TimeLimits};
use crate::transposition::{Bounds, TranspositionTable};
use std::cmp::{max, min};

/// Positive infinity for alpha-beta bounds.
///
/// Must be strictly greater in magnitude than any score the search can produce,
/// including checkmate scores, so that `SearchResult::worst()` is truly the
/// lowest possible value.
const INFINITY: i32 = 1_000_000;

/// Negative infinity for alpha-beta bounds
const NEGATIVE_INFINITY: i32 = -INFINITY;

/// Checkmate score (before mate-distance adjustment). Leaves room above it for
/// `INFINITY` and below the maximum reachable evaluation.
const CHECKMATE_SCORE: i32 = 100_000;

/// Scores whose magnitude is at least this are treated as mate scores for the
/// purpose of mate-distance (ply) normalization. `MAX_DEPTH` plies of slack is
/// far more than the search ever reaches.
const MATE_THRESHOLD: i32 = CHECKMATE_SCORE - 1000;

/// Maximum number of plies quiescence search may extend.
///
/// Quiescence follows captures, promotions, and checks, so a forcing sequence
/// of checks/evasions can otherwise recurse without bound. This caps that
/// recursion; real tactical sequences resolve far sooner.
const MAX_QUIESCENCE_DEPTH: u8 = 32;

/// Hard ceiling on search ply, bounding recursion when extensions keep the
/// remaining depth from decreasing.
const MAX_PLY: u8 = 100;

/// Number of reusable per-ply move buffers. Quiescence extends past `MAX_PLY`,
/// so this leaves room for the deepest quiescence ply plus a small margin.
const MAX_SEARCH_PLY: usize = MAX_PLY as usize + MAX_QUIESCENCE_DEPTH as usize + 8;

/// Depth reduction applied by null-move pruning.
const NULL_MOVE_REDUCTION: u8 = 3;

/// Minimum remaining depth at which null-move pruning is attempted.
const NULL_MOVE_MIN_DEPTH: u8 = 3;

/// Minimum remaining depth at which late move reductions are applied.
const LMR_MIN_DEPTH: u8 = 3;

/// Move index from which late move reductions begin.
const LMR_MIN_MOVE_INDEX: usize = 3;

/// Dimensions of the precomputed late-move-reduction table.
const LMR_TABLE_SIZE: usize = 64;

/// Maximum remaining depth at which reverse futility pruning is applied.
const RFP_MAX_DEPTH: u8 = 6;

/// Per-ply margin for reverse futility pruning, in centipawns.
const RFP_MARGIN: i32 = 80;

/// Minimum remaining depth at which aspiration windows are used.
const ASPIRATION_MIN_DEPTH: u8 = 5;

/// Initial half-width of the aspiration window in centipawns.
const ASPIRATION_INITIAL_DELTA: i32 = 30;

/// Multiplier that shifts an ordering key left to make room for a move's
/// original index, so equal keys break ties in generation order. Must exceed
/// the maximum number of pseudo-legal moves in any position.
const ORDER_INDEX_SCALE: i64 = 1024;

/// Slots in the counter-move table, indexed by `from * 64 + to` of the parent
/// move (64 squares squared).
const COUNTER_MOVE_SLOTS: usize = 64 * 64;

/// Ordering key for a counter move: just behind killer moves, ahead of
/// promotions and history-ordered quiets.
const COUNTER_MOVE_KEY: i32 = -450;

/// Index into the counter-move table for a parent move.
#[inline]
fn counter_index(mv: Move) -> usize {
    (mv.from as usize) * 64 + mv.to as usize
}

/// Most Valuable Victim - Least Valuable Attacker scores for move ordering.
///
/// Indexed `[victim][attacker]` using `Piece::index()` ordering
/// (Pawn, Knight, Bishop, Rook, Queen, King). A more valuable victim dominates,
/// and among equal victims a less valuable attacker scores higher.
pub const MVV_LVA_SCORES: [[i8; 6]; 6] = [
    // victim Pawn   (attacker P,  N,  B,  R,  Q,  K)
    [15, 14, 13, 12, 11, 10],
    // victim Knight
    [25, 24, 23, 22, 21, 20],
    // victim Bishop
    [35, 34, 33, 32, 31, 30],
    // victim Rook
    [45, 44, 43, 42, 41, 40],
    // victim Queen
    [55, 54, 53, 52, 51, 50],
    // victim King (capture should never happen)
    [0, 0, 0, 0, 0, 0],
];

/// The main chess position searcher.
pub struct Searcher {
    move_generator: MoveGenerator,
    evaluator: Evaluator,
    transposition_table: TranspositionTable,
    killer_moves: KillerMoves,
    timer: SearchTimer,
    repetition: RepetitionTable,
    history: HistoryTable,
    /// Reusable move buffers, one per ply, to avoid per-node allocation.
    move_buffers: Vec<Vec<Move>>,
    /// Reusable per-ply ordering-key buffers, parallel to `move_buffers`, so
    /// move ordering need not allocate.
    order_scores: Vec<Vec<i64>>,
    /// Counter-move heuristic: the quiet move that most recently refuted the
    /// move made at the parent node, indexed by `[prev.from][prev.to]`. Ordered
    /// early since a refutation of the same move often refutes it again.
    counter_moves: Vec<Option<Move>>,
    /// Precomputed reductions indexed by [remaining depth][move index].
    lmr_table: [[u8; LMR_TABLE_SIZE]; LMR_TABLE_SIZE],
}

impl Searcher {
    /// Creates a new searcher with all components initialized
    pub fn new() -> Self {
        Self {
            move_generator: MoveGenerator::new(),
            evaluator: Evaluator::new(),
            transposition_table: TranspositionTable::new(),
            killer_moves: KillerMoves::new(),
            timer: SearchTimer::new(),
            repetition: RepetitionTable::new(),
            history: HistoryTable::new(),
            move_buffers: vec![Vec::new(); MAX_SEARCH_PLY],
            order_scores: vec![Vec::new(); MAX_SEARCH_PLY],
            counter_moves: vec![None; COUNTER_MOVE_SLOTS],
            lmr_table: Self::build_lmr_table(),
        }
    }

    /// Builds the late-move-reduction table with a logarithmic growth so that
    /// reductions increase gently with both remaining depth and move index.
    fn build_lmr_table() -> [[u8; LMR_TABLE_SIZE]; LMR_TABLE_SIZE] {
        let mut table = [[0u8; LMR_TABLE_SIZE]; LMR_TABLE_SIZE];
        for depth in 1..LMR_TABLE_SIZE {
            for move_index in 1..LMR_TABLE_SIZE {
                let reduction =
                    0.75 + (depth as f64).ln() * (move_index as f64).ln() / 2.25;
                table[depth][move_index] = reduction as u8;
            }
        }
        table
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
    /// Nodes searched in the most recent `find_best_move` call.
    pub fn last_search_nodes(&self) -> u64 {
        self.timer.nodes()
    }

    pub fn find_best_move(
        &mut self,
        board: &Board,
        max_depth: u8,
        time_limit: Option<TimeLimits>,
    ) -> (i32, Option<Move>) {
        self.timer.start_with_limits(time_limit);
        self.history.age();
        self.transposition_table.new_search();

        let mut best_score = NEGATIVE_INFINITY;
        let mut best_move = None;

        for current_depth in 1..=max_depth {
            let result = self.aspiration_search(board, current_depth, best_score);

            // Always accept depth 1 so we never return without a legal move,
            // even with a zero or already-expired time budget. Deeper iterations
            // are only committed if the timer did not abort them mid-search.
            if current_depth == 1 || !self.timer.should_stop() {
                best_score = result.score;
                best_move = result.best_move;

                self.cache_search_result(board, &result, current_depth);
                self.timer
                    .print_info(current_depth, result.score, result.best_move);
            }

            // Stop on the hard limit (aborted mid-iteration) or once the soft
            // limit has passed (no point starting a deeper iteration).
            if self.timer.should_stop() || self.timer.soft_expired() {
                break;
            }
        }

        (best_score, best_move)
    }

    /// Searches a depth using a narrow window around the previous score,
    /// widening and re-searching whenever the result falls outside it.
    fn aspiration_search(&mut self, board: &Board, depth: u8, prev_score: i32) -> SearchResult {
        if depth < ASPIRATION_MIN_DEPTH || prev_score.abs() >= MATE_THRESHOLD {
            return self.search_position(board, depth, NEGATIVE_INFINITY, INFINITY);
        }

        let mut delta = ASPIRATION_INITIAL_DELTA;
        let mut alpha = prev_score - delta;
        let mut beta = prev_score + delta;

        loop {
            let result = self.search_position(board, depth, alpha, beta);

            if self.timer.should_stop() {
                return result;
            }

            if result.score <= alpha {
                alpha = (alpha - delta).max(NEGATIVE_INFINITY);
                delta *= 2;
            } else if result.score >= beta {
                beta = (beta + delta).min(INFINITY);
                delta *= 2;
            } else {
                return result;
            }
        }
    }

    /// Searches a position to a given depth using negamax with alpha-beta.
    fn search_position(&mut self, board: &Board, depth: u8, alpha: i32, beta: i32) -> SearchResult {
        // Repetition history is seeded with the game line by the UCI layer and
        // extended/unwound by `negamax` itself, so nothing to push here.
        self.negamax(board, depth, 0, alpha, beta, None, SearchContext::new())
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
        prev_move: Option<Move>,
        mut context: SearchContext,
    ) -> SearchResult {
        self.timer.increment_nodes();

        if ply >= MAX_PLY {
            return SearchResult::new(self.evaluator.evaluate(board, &self.move_generator.lookup, alpha, beta), None);
        }

        let original_alpha = alpha;
        let hash = board.hash;

        // A position seen earlier on the search path (or in the game history) is
        // a draw. The root (ply 0) is never compared against itself.
        if ply > 0 && self.repetition.is_repetition(hash) {
            return SearchResult::new(0, None);
        }

        // Check if we've already seen this position
        if let Some(cached_result) =
            self.probe_transposition_table(hash, depth, ply, alpha, beta, &mut context)
        {
            return cached_result;
        }

        // Quiescence search checks, captures, and promotions
        if depth == 0 {
            let score = self.search_until_quiet(board, alpha, beta, ply, 0);
            return SearchResult::new(score, None);
        }

        let in_check = self.move_generator.is_in_check(board);

        // Reverse futility pruning: near the leaves, a static evaluation a clear
        // margin above beta is unlikely to fall below it, so fail high early.
        if !in_check && depth <= RFP_MAX_DEPTH && beta.abs() < MATE_THRESHOLD {
            // RFP needs an accurate static eval, so force the full evaluation
            // (a wide window disables the lazy-eval shortcut).
            let static_eval = self.evaluator.evaluate(
                board,
                &self.move_generator.lookup,
                NEGATIVE_INFINITY,
                INFINITY,
            );
            if static_eval - RFP_MARGIN * depth as i32 >= beta {
                return SearchResult::new(static_eval, None);
            }
        }

        // Null-move pruning: if giving the opponent a free move still fails high,
        // the position is good enough to prune. Skipped at the root, in check, in
        // likely-zugzwang positions, and after another null move.
        if context.allow_null
            && ply > 0
            && !in_check
            && depth >= NULL_MOVE_MIN_DEPTH
            && self.has_non_pawn_material(board)
        {
            let null_board = board.make_null_move();
            let reduced_depth = depth.saturating_sub(1 + NULL_MOVE_REDUCTION);
            let score = -self
                .negamax(
                    &null_board,
                    reduced_depth,
                    ply + 1,
                    -beta,
                    -beta + 1,
                    None,
                    SearchContext::null_disallowed(),
                )
                .score;

            if score >= beta {
                return SearchResult::new(beta, None);
            }
        }

        // Generate and order moves (best moves first for better pruning) into
        // the reusable buffer for this ply.
        let mut moves = std::mem::take(&mut self.move_buffers[ply as usize]);
        self.move_generator.generate_moves_into(board, &mut moves);

        // Check for checkmate/stalemate
        if moves.is_empty() {
            self.move_buffers[ply as usize] = moves;
            return self.handle_terminal_position(in_check, ply);
        }

        let counter_move = prev_move.and_then(|prev| self.counter_moves[counter_index(prev)]);

        let mut scores = std::mem::take(&mut self.order_scores[ply as usize]);
        self.score_moves(board, &moves, context.tt_best_move, counter_move, ply, &mut scores);

        let mut best_result = SearchResult::worst(moves[0]);

        // Make the current position visible to deeper nodes for repetition
        // detection, then unwind it once the subtree is searched.
        self.repetition.push(hash);

        let mut move_index = 0;
        while move_index < moves.len() {
            if self.timer.should_stop() {
                break;
            }

            // Bring the best-scoring remaining move to `move_index` on demand;
            // a cutoff lets us skip ordering the rest entirely.
            Self::select_next_move(&mut moves, &mut scores, move_index);
            let current_move = moves[move_index];

            let next_position = board.clone_with_move(&current_move);
            let gives_check = self.move_generator.is_in_check(&next_position);
            let extension = (gives_check && ply < MAX_PLY) as u8;
            let new_depth = depth - 1 + extension;

            // Principal variation search: the first move is searched with the
            // full window; the rest with a null window (optionally reduced) and
            // only re-searched if they beat alpha.
            let child = Some(current_move);
            let score = if move_index == 0 {
                -self
                    .negamax(&next_position, new_depth, ply + 1, -beta, -alpha, child, SearchContext::new())
                    .score
            } else {
                let reduction =
                    self.late_move_reduction(depth, move_index, current_move, in_check, gives_check);
                let reduced_depth = if reduction > 0 {
                    new_depth.saturating_sub(reduction).max(1)
                } else {
                    new_depth
                };

                let mut score = -self
                    .negamax(
                        &next_position,
                        reduced_depth,
                        ply + 1,
                        -alpha - 1,
                        -alpha,
                        child,
                        SearchContext::new(),
                    )
                    .score;

                // A reduced search that beats alpha is re-tried at full depth.
                if reduction > 0 && score > alpha {
                    score = -self
                        .negamax(
                            &next_position,
                            new_depth,
                            ply + 1,
                            -alpha - 1,
                            -alpha,
                            child,
                            SearchContext::new(),
                        )
                        .score;
                }

                // A null-window score inside the window needs a full re-search.
                if score > alpha && score < beta {
                    score = -self
                        .negamax(
                            &next_position,
                            new_depth,
                            ply + 1,
                            -beta,
                            -alpha,
                            child,
                            SearchContext::new(),
                        )
                        .score;
                }

                score
            };

            if score > best_result.score {
                best_result.score = score;
                best_result.best_move = Some(current_move);
            }

            alpha = max(alpha, score);
            if alpha >= beta {
                if current_move.move_type == MoveType::Quiet {
                    self.killer_moves.store(current_move, ply);
                    self.history.record_cutoff(&current_move, depth);
                    if let Some(prev) = prev_move {
                        self.counter_moves[counter_index(prev)] = Some(current_move);
                    }
                }
                break;
            }

            move_index += 1;
        }

        self.repetition.pop();
        self.order_scores[ply as usize] = scores;
        self.move_buffers[ply as usize] = moves;

        // Once the timer has fired the move loop may have exited early, leaving a
        // partial (unreliable) result. Storing it would pollute the table for
        // later searches, so only cache results from searches that ran to
        // completion.
        if !self.timer.should_stop() {
            let bound = self.determine_bound(best_result.score, original_alpha, beta);
            self.store_in_transposition_table(hash, &best_result, depth, ply, bound);
        }

        best_result
    }

    /// Reduction applied by late move reductions for a given move.
    ///
    /// Only late, quiet, non-checking moves are reduced, and never while in
    /// check. Returns 0 when no reduction should be applied.
    fn late_move_reduction(
        &self,
        depth: u8,
        move_index: usize,
        mv: Move,
        in_check: bool,
        gives_check: bool,
    ) -> u8 {
        if depth < LMR_MIN_DEPTH
            || move_index < LMR_MIN_MOVE_INDEX
            || in_check
            || gives_check
            || mv.move_type != MoveType::Quiet
        {
            return 0;
        }

        let depth_index = (depth as usize).min(LMR_TABLE_SIZE - 1);
        let move_index = move_index.min(LMR_TABLE_SIZE - 1);
        self.lmr_table[depth_index][move_index]
    }

    /// Returns true if the side to move has a piece other than pawns and the
    /// king, used to avoid null-move pruning in likely-zugzwang positions.
    fn has_non_pawn_material(&self, board: &Board) -> bool {
        let color = board.active_color();
        (board.bb(color, Piece::Knight)
            | board.bb(color, Piece::Bishop)
            | board.bb(color, Piece::Rook)
            | board.bb(color, Piece::Queen))
            != 0
    }

    /// Searches until position is "quiet" (no captures, checks, or promotions)
    ///
    /// This prevents the "horizon effect" where the engine stops searching right
    /// before a capture sequence, leading to bad evaluations.
    fn search_until_quiet(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        ply: u8,
        qdepth: u8,
    ) -> i32 {
        self.timer.increment_nodes();

        // Bail out of pathological forcing sequences (e.g. perpetual checks)
        // with a static evaluation to keep recursion bounded.
        if qdepth >= MAX_QUIESCENCE_DEPTH {
            return self.evaluator.evaluate(board, &self.move_generator.lookup, alpha, beta);
        }

        let currently_in_check = self.move_generator.is_in_check(board);

        let mut moves = std::mem::take(&mut self.move_buffers[ply as usize]);
        if currently_in_check {
            self.move_generator.generate_moves_into(board, &mut moves);
        } else {
            self.move_generator
                .generate_quiescence_moves_into(board, &mut moves);
        }

        let mut scores = std::mem::take(&mut self.order_scores[ply as usize]);
        self.score_captures(board, &moves, &mut scores);

        // Checkmate detection
        if moves.is_empty() && currently_in_check {
            self.order_scores[ply as usize] = scores;
            self.move_buffers[ply as usize] = moves;
            return -CHECKMATE_SCORE + ply as i32;
        }

        // When in check we cannot "stand pat" - every evasion must be searched,
        // and failing to escape means we are being mated. Only use the static
        // evaluation as a lower bound when the side to move is not in check.
        if !currently_in_check {
            let stand_pat = self.evaluator.evaluate(board, &self.move_generator.lookup, alpha, beta);
            if stand_pat >= beta {
                self.order_scores[ply as usize] = scores;
                self.move_buffers[ply as usize] = moves;
                return beta;
            }
            alpha = max(alpha, stand_pat);
        }

        let mut cutoff = false;
        let mut move_index = 0;
        while move_index < moves.len() {
            if self.timer.should_stop() {
                break;
            }

            Self::select_next_move(&mut moves, &mut scores, move_index);
            let mv = moves[move_index];
            move_index += 1;

            // Skip captures that lose material on the exchange. Evasions are
            // searched in full, so this only prunes when not in check.
            if !currently_in_check
                && (mv.move_type == MoveType::Capture || mv.move_type == MoveType::EnPassant)
                && self.move_generator.see(board, &mv) < 0
            {
                continue;
            }

            let next_position = board.clone_with_move(&mv);
            let score =
                -self.search_until_quiet(&next_position, -beta, -alpha, ply + 1, qdepth + 1);

            if score >= beta {
                cutoff = true;
                break;
            }

            alpha = max(alpha, score);
        }

        self.order_scores[ply as usize] = scores;
        self.move_buffers[ply as usize] = moves;
        if cutoff {
            beta
        } else {
            alpha
        }
    }

    #[cfg(test)]
    fn is_draw_by_repetition(&self, board: &Board) -> bool {
        let current_hash = board.hash;
        self.repetition.is_repetition(current_hash)
    }

    /// Seeds the repetition history with the positions already played in the
    /// game. Called by the UCI layer for each `position` command.
    pub fn reset_repetition(&mut self) {
        self.repetition = RepetitionTable::new();
    }

    /// Records a position that occurred in the actual game line.
    pub fn record_game_position(&mut self, board: &Board) {
        self.repetition.push(board.hash);
    }

    /// Checks if we've already searched this position
    fn probe_transposition_table(
        &self,
        position_hash: u64,
        depth: u8,
        ply: u8,
        mut alpha: i32,
        mut beta: i32,
        context: &mut SearchContext,
    ) -> Option<SearchResult> {
        let entry = self.transposition_table.retrieve(position_hash)?;

        // Store TT move for move ordering even if depth is insufficient
        context.tt_best_move = entry.best_move;

        // Only use entry if it was searched to sufficient depth
        if entry.depth < depth {
            return None;
        }

        // Mate scores are stored relative to the entry's node; rebase to this ply.
        let eval = Self::score_from_tt(entry.eval, ply);

        match entry.bounds {
            Bounds::Exact => {
                return Some(SearchResult::new(eval, entry.best_move));
            }
            Bounds::Lower => {
                alpha = max(alpha, eval);
            }
            Bounds::Upper => {
                beta = min(beta, eval);
            }
        }

        if alpha >= beta {
            return Some(SearchResult::new(eval, entry.best_move));
        }

        // Can't use this entry
        None
    }

    /// Stores a search result in the transposition table.
    fn store_in_transposition_table(
        &mut self,
        position_hash: u64,
        result: &SearchResult,
        depth: u8,
        ply: u8,
        bound: Bounds,
    ) {
        // Store mate scores relative to this node so they stay correct when the
        // entry is reused at a different distance from the root.
        let eval = Self::score_to_tt(result.score, ply);
        self.transposition_table
            .store(position_hash, eval, result.best_move, depth, bound);
    }

    /// Caches the result from iterative deepening for move ordering.
    fn cache_search_result(&mut self, board: &Board, result: &SearchResult, depth: u8) {
        let hash = board.hash;
        self.store_in_transposition_table(hash, result, depth, 0, Bounds::Exact);
    }

    /// Adjusts a mate score retrieved from the TT to be relative to the current ply.
    fn score_from_tt(score: i32, ply: u8) -> i32 {
        if score > MATE_THRESHOLD {
            score - ply as i32
        } else if score < -MATE_THRESHOLD {
            score + ply as i32
        } else {
            score
        }
    }

    /// Adjusts a mate score to be relative to the stored node before caching.
    fn score_to_tt(score: i32, ply: u8) -> i32 {
        if score > MATE_THRESHOLD {
            score + ply as i32
        } else if score < -MATE_THRESHOLD {
            score - ply as i32
        } else {
            score
        }
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
    fn handle_terminal_position(&self, in_check: bool, ply: u8) -> SearchResult {
        if in_check {
            // Score relative to distance from the root so shorter mates are
            // preferred (a mate closer to the root is worth more).
            let mate_score = -CHECKMATE_SCORE + ply as i32;
            SearchResult::checkmate(mate_score)
        } else {
            SearchResult::stalemate()
        }
    }

    /// Computes an ordering key for every move so the search can select the
    /// best-scoring move on demand instead of sorting the whole list up front.
    ///
    /// Lower keys are searched first. The move's original index is folded into
    /// the low bits so ties resolve in generation order, making the on-demand
    /// selection reproduce a stable sort exactly.
    ///
    /// Priority:
    /// 1. Transposition table move
    /// 2. Captures (MVV-LVA)
    /// 3. Killer moves
    /// 4. Promotions
    /// 5. History heuristic
    /// 6. Other moves
    fn score_moves(
        &self,
        board: &Board,
        moves: &[Move],
        tt_move: Option<Move>,
        counter_move: Option<Move>,
        ply: u8,
        scores: &mut Vec<i64>,
    ) {
        scores.clear();
        for (index, mv) in moves.iter().enumerate() {
            let key = self.move_order_key(board, mv, tt_move, counter_move, ply);
            scores.push((key as i64) * ORDER_INDEX_SCALE + index as i64);
        }
    }

    /// The base ordering key for a single move (see `score_moves`).
    fn move_order_key(
        &self,
        board: &Board,
        mv: &Move,
        tt_move: Option<Move>,
        counter_move: Option<Move>,
        ply: u8,
    ) -> i32 {
        if let Some(best_move) = tt_move {
            if *mv == best_move {
                return i32::MIN;
            }
        }

        if mv.move_type == MoveType::Capture || mv.move_type == MoveType::EnPassant {
            if let Some(score) = self.calculate_capture_score(board, mv) {
                return -(score as i32) - 1000;
            }
        }

        if self.killer_moves.is_killer(mv, ply) {
            return -500;
        }

        if Some(*mv) == counter_move {
            return COUNTER_MOVE_KEY;
        }

        if mv.move_type == MoveType::Promotion {
            return -400;
        }

        if mv.move_type == MoveType::Quiet {
            return -self.history.get_score(mv);
        }

        0
    }

    /// Swaps the lowest-key (best) remaining move into position `start`.
    fn select_next_move(moves: &mut [Move], scores: &mut [i64], start: usize) {
        let mut best = start;
        for i in (start + 1)..moves.len() {
            if scores[i] < scores[best] {
                best = i;
            }
        }
        moves.swap(start, best);
        scores.swap(start, best);
    }

    /// Computes MVV-LVA ordering keys for quiescence moves, with the original
    /// index folded in so on-demand selection matches a stable sort.
    fn score_captures(&self, board: &Board, moves: &[Move], scores: &mut Vec<i64>) {
        scores.clear();
        for (index, mv) in moves.iter().enumerate() {
            let key = if mv.move_type == MoveType::EnPassant {
                -10
            } else {
                self.calculate_capture_score(board, mv)
                    .map(|score| -(score as i32))
                    .unwrap_or(0)
            };
            scores.push((key as i64) * ORDER_INDEX_SCALE + index as i64);
        }
    }

    /// Calculates the capture score for MVV-LVA ordering
    fn calculate_capture_score(&self, board: &Board, mv: &Move) -> Option<i8> {
        let attacker = board.get_piece_at(mv.from)?;
        let victim = board.get_piece_at(mv.to)?;

        Some(MVV_LVA_SCORES[victim.index()][attacker.index()])
    }

    /// Updates position repetition (for repetition detection)
    #[allow(dead_code)]
    fn push_position(&mut self, board: &Board) {
        self.repetition.push(board.hash);
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

    fn worst(mv: Move) -> Self {
        Self {
            score: NEGATIVE_INFINITY,
            best_move: Some(mv),
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
    allow_null: bool,
}

impl SearchContext {
    fn new() -> Self {
        Self {
            tt_best_move: None,
            allow_null: true,
        }
    }

    /// Context for the subtree after a null move, where a further null move is
    /// disallowed.
    fn null_disallowed() -> Self {
        Self {
            tt_best_move: None,
            allow_null: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Deep enough that late move reductions don't postpone quiet mating moves
    // past the search horizon for these positions.
    const SEARCH_DEPTH: u8 = 8;

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

    /// Asserts the engine finds a decisively winning continuation. Used where
    /// reverse futility pruning may prefer an immediate material win over a
    /// slightly longer forced mate (both are crushing).
    fn assert_finds_winning_move(fen: &str, min_score: i32) {
        let board = Board::new(fen);
        let mut searcher = Searcher::new();
        let (score, best_move) = searcher.find_best_move(&board, SEARCH_DEPTH, None);

        assert!(best_move.is_some(), "Engine should find a move");
        assert!(
            score >= min_score,
            "Expected a winning score >= {}, got {}",
            min_score,
            score
        );
    }

    #[test]
    fn test_king_move_while_in_check() {
        assert_finds_move(
            "r3k3/p1R2Qp1/2pq4/4p3/2P4P/3BP3/P4P1P/5bK1 b q - 0 1",
            "e8d8",
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
        // With reverse futility pruning the engine takes the immediate winning
        // capture rather than the longer forced mate; both are decisive.
        assert_finds_winning_move("1k2r3/pP3pp1/8/3P1B1p/5q2/N1P2b2/PP3Pp1/R5K1 b - - 0 1", 500);
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
    fn infinity_exceeds_mate_scores() {
        // The core invariant behind fix #1: -INFINITY must be lower than any
        // reachable score (including being mated) so SearchResult::worst() works.
        assert!(NEGATIVE_INFINITY < -CHECKMATE_SCORE);
        assert!(INFINITY > CHECKMATE_SCORE);
        assert!(CHECKMATE_SCORE > MATE_THRESHOLD);
    }

    #[test]
    fn winning_mate_has_mate_score() {
        let board = Board::new("4k3/5p2/8/6B1/8/8/8/3R2K1 w - - 0 1");
        let mut searcher = Searcher::new();
        let (score, _) = searcher.find_best_move(&board, 4, None);
        assert!(
            score > MATE_THRESHOLD,
            "expected a mate score for a winning position, got {}",
            score
        );
    }

    #[test]
    fn mvv_lva_orders_captures_correctly() {
        use crate::pieces::Piece;
        let pxq = MVV_LVA_SCORES[Piece::Queen.index()][Piece::Pawn.index()];
        let qxp = MVV_LVA_SCORES[Piece::Pawn.index()][Piece::Queen.index()];
        let pxn = MVV_LVA_SCORES[Piece::Knight.index()][Piece::Pawn.index()];
        let rxq = MVV_LVA_SCORES[Piece::Queen.index()][Piece::Rook.index()];

        // More valuable victim dominates.
        assert!(pxq > qxp, "PxQ should rank above QxP");
        assert!(pxq > pxn, "queen victim should rank above knight victim");
        // Among the same victim, the cheaper attacker ranks higher.
        assert!(pxq > rxq, "PxQ should rank above RxQ");
    }

    #[test]
    fn records_game_positions_for_repetition() {
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let mut searcher = Searcher::new();

        searcher.reset_repetition();
        searcher.record_game_position(&board);
        searcher.record_game_position(&board);

        // The seeded game line now makes this position a repetition draw.
        assert!(searcher.is_draw_by_repetition(&board));

        // A new position command resets the history.
        searcher.reset_repetition();
        assert!(!searcher.is_draw_by_repetition(&board));
    }

    #[test]
    fn returns_legal_move_with_zero_time() {
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let mut searcher = Searcher::new();

        let (_, best_move) =
            searcher.find_best_move(&board, 64, Some(TimeLimits::fixed(Duration::from_millis(0))));

        assert!(
            best_move.is_some(),
            "engine must return a legal move even with a zero time budget"
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
        searcher.find_best_move(&board, 10, Some(TimeLimits::fixed(time_limit)));
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
