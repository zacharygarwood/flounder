use crate::board::Board;
use crate::pieces::{Color, Piece, COLOR_COUNT};

/// Slots per side to move. A power of two so the index is a mask of the key.
const CORRECTION_SLOTS: usize = 1 << 14;

/// Corrections are stored in 1/GRAIN centipawn units so small, repeated updates
/// are not lost to integer rounding.
const CORRECTION_GRAIN: i32 = 256;

/// Largest correction applied, in centipawns. Bounds the damage when a pawn
/// structure's slot is shared by unrelated positions.
const MAX_CORRECTION: i32 = 64;

/// Weights of an update are out of this; deeper searches get more weight, up to
/// `MAX_UPDATE_WEIGHT`.
const UPDATE_WEIGHT_SCALE: i32 = 256;
const MAX_UPDATE_WEIGHT: i32 = 16;

/// Static-eval correction history keyed by pawn structure.
///
/// A handcrafted eval is systematically wrong in some pawn structures (it
/// cannot see every plan they imply). This learns, from finished searches, how
/// far the search result tends to land from the static eval for the current
/// pawn structure and side to move, and shifts the static eval by that much so
/// eval-based pruning decides on a more accurate number.
pub struct CorrectionHistory {
    table: Vec<i32>,
}

impl CorrectionHistory {
    pub fn new() -> Self {
        Self {
            table: vec![0; COLOR_COUNT * CORRECTION_SLOTS],
        }
    }

    /// `raw_eval` adjusted by what earlier searches learned about this pawn
    /// structure, from the side to move's perspective.
    pub fn corrected(&self, board: &Board, raw_eval: i32) -> i32 {
        raw_eval + self.table[Self::index(board)] / CORRECTION_GRAIN
    }

    /// Records that a search of `depth` found `search_score` where the static
    /// eval said `raw_eval`. Callers should only pass scores that measure the
    /// eval's error: exact or bound-consistent, not mates, not from a capture.
    pub fn update(&mut self, board: &Board, depth: u8, raw_eval: i32, search_score: i32) {
        let entry = &mut self.table[Self::index(board)];
        let target = (search_score - raw_eval) * CORRECTION_GRAIN;
        let weight = (depth as i32 + 1).min(MAX_UPDATE_WEIGHT);
        let blended = (*entry * (UPDATE_WEIGHT_SCALE - weight) + target * weight) / UPDATE_WEIGHT_SCALE;
        let limit = MAX_CORRECTION * CORRECTION_GRAIN;
        *entry = blended.clamp(-limit, limit);
    }

    fn index(board: &Board) -> usize {
        let side = board.active_color().index();
        let key = pawn_key(
            board.bb(Color::White, Piece::Pawn),
            board.bb(Color::Black, Piece::Pawn),
        );
        side * CORRECTION_SLOTS + (key as usize & (CORRECTION_SLOTS - 1))
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Hashes the pawn bitboards directly. Recomputing this is a couple of
/// multiplies, cheaper than maintaining a separate incremental pawn key.
fn pawn_key(white_pawns: u64, black_pawns: u64) -> u64 {
    let mixed = white_pawns.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ black_pawns.rotate_left(32).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    mixed ^ (mixed >> 29)
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn unseen_structure_is_not_corrected() {
        let history = CorrectionHistory::new();
        assert_eq!(history.corrected(&Board::new(START), 50), 50);
    }

    #[test]
    fn search_scores_above_eval_raise_the_correction() {
        let mut history = CorrectionHistory::new();
        let board = Board::new(START);
        for _ in 0..50 {
            history.update(&board, 10, 0, 40);
        }
        let corrected = history.corrected(&board, 0);
        assert!(corrected > 20 && corrected <= 40, "corrected = {corrected}");
    }

    #[test]
    fn correction_is_capped() {
        let mut history = CorrectionHistory::new();
        let board = Board::new(START);
        for _ in 0..1000 {
            history.update(&board, 20, 0, 5000);
        }
        assert_eq!(history.corrected(&board, 0), MAX_CORRECTION);
    }

    #[test]
    fn correction_is_kept_per_side_to_move() {
        let mut history = CorrectionHistory::new();
        let white_to_move = Board::new(START);
        let black_to_move = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
        for _ in 0..50 {
            history.update(&white_to_move, 10, 0, 40);
        }
        assert_eq!(history.corrected(&black_to_move, 0), 0);
    }
}
