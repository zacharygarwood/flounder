use crate::{
    bitboard::SQUARES,
    moves::Move,
    pieces::{Color, COLOR_COUNT},
};

const SQUARE_COUNT: usize = SQUARES as usize;

/// Bound on the magnitude of a history score. The gravity update keeps every
/// score within `[-MAX_HISTORY, MAX_HISTORY]`.
pub const MAX_HISTORY: i32 = 16_384;

/// Largest bonus or penalty applied by a single update.
const MAX_HISTORY_BONUS: i32 = 1_200;

/// History heuristic table for move ordering
///
/// The history heuristic tracks which quiet moves have historically
/// caused beta cutoffs. Moves that frequently cause cutoffs are likely
/// to be good in similar positions and should be searched earlier.
/// Scores are kept per side to move, since the same from/to squares mean
/// different things for White and Black.
#[derive(Debug, Clone)]
pub struct HistoryTable {
    scores: [[[i32; SQUARE_COUNT]; SQUARE_COUNT]; COLOR_COUNT],
}

impl HistoryTable {
    /// Creates a new history table
    pub fn new() -> Self {
        Self {
            scores: [[[0; SQUARE_COUNT]; SQUARE_COUNT]; COLOR_COUNT],
        }
    }

    /// The bonus for a cutoff at `depth`, and the penalty for a quiet move
    /// searched before it. Deeper cutoffs are more significant.
    pub fn bonus(depth: u8) -> i32 {
        let depth = depth as i32;
        (16 * depth * depth + 32 * depth).min(MAX_HISTORY_BONUS)
    }

    /// Adjusts a move's score by `bonus` (negative for a penalty).
    ///
    /// Uses a "gravity" update: the further a score already is in the
    /// direction of the bonus, the less it moves, so scores stay bounded by
    /// `MAX_HISTORY` and recent results outweigh stale ones.
    ///
    /// # Arguments
    /// * `color` - The side that made the move
    /// * `mv` - The move to update
    /// * `bonus` - Positive to reward the move, negative to penalize it
    pub fn update(&mut self, color: Color, mv: &Move, bonus: i32) {
        let score = &mut self.scores[color as usize][mv.from as usize][mv.to as usize];
        *score += bonus - *score * bonus.abs() / MAX_HISTORY;
    }

    /// Gets the history score for a move
    ///
    /// # Arguments
    /// * `color` - The side making the move
    /// * `mv` - The move to get the score for
    ///
    /// # Returns
    /// The history score
    pub fn get_score(&self, color: Color, mv: &Move) -> i32 {
        self.scores[color as usize][mv.from as usize][mv.to as usize]
    }

    /// Ages all history scores by dividing by 2
    pub fn age(&mut self) {
        for score in self.scores.iter_mut().flatten().flatten() {
            *score /= 2;
        }
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::moves::MoveType;
    use crate::pieces::Piece;

    use super::*;

    fn create_test_move(from: u8, to: u8) -> Move {
        Move {
            from,
            to,
            move_type: MoveType::Quiet,
            piece_type: Piece::Pawn,
        }
    }

    #[test]
    fn test_new_table_has_zero_scores() {
        let history = HistoryTable::new();
        let mv = create_test_move(12, 28);
        assert_eq!(history.get_score(Color::White, &mv), 0);
    }

    #[test]
    fn test_bonus_increases_and_penalty_decreases_score() {
        let mut history = HistoryTable::new();
        let mv = create_test_move(12, 28);

        history.update(Color::White, &mv, 500);
        assert_eq!(history.get_score(Color::White, &mv), 500);

        history.update(Color::White, &mv, -500);
        assert!(history.get_score(Color::White, &mv) < 500);
    }

    #[test]
    fn test_deeper_cutoffs_earn_larger_bonus() {
        assert!(HistoryTable::bonus(10) > HistoryTable::bonus(5));
        assert!(HistoryTable::bonus(60) <= MAX_HISTORY_BONUS);
    }

    #[test]
    fn test_scores_stay_bounded() {
        let mut history = HistoryTable::new();
        let mv = create_test_move(12, 28);

        for _ in 0..10_000 {
            history.update(Color::White, &mv, MAX_HISTORY_BONUS);
        }
        assert!(history.get_score(Color::White, &mv) <= MAX_HISTORY);

        for _ in 0..10_000 {
            history.update(Color::White, &mv, -MAX_HISTORY_BONUS);
        }
        assert!(history.get_score(Color::White, &mv) >= -MAX_HISTORY);
    }

    #[test]
    fn test_colors_and_moves_have_independent_scores() {
        let mut history = HistoryTable::new();
        let mv1 = create_test_move(12, 28);
        let mv2 = create_test_move(6, 21);

        history.update(Color::White, &mv1, 400);

        assert_eq!(history.get_score(Color::White, &mv1), 400);
        assert_eq!(history.get_score(Color::Black, &mv1), 0);
        assert_eq!(history.get_score(Color::White, &mv2), 0);
    }

    #[test]
    fn test_age_reduces_scores() {
        let mut history = HistoryTable::new();
        let mv = create_test_move(12, 28);

        history.update(Color::White, &mv, 400);
        history.age();
        assert_eq!(history.get_score(Color::White, &mv), 200);

        history.age();
        assert_eq!(history.get_score(Color::White, &mv), 100);
    }
}
