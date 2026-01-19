use crate::{bitboard::SQUARES, moves::Move};

const SQUARE_COUNT: usize = SQUARES as usize;

/// History heuristic table for move ordering
///
/// The history heuristic tracks which quiet moves have historically
/// caused beta cutoffs. Moves that frequently cause cutoffs are likely
/// to be good in similar positions and should be searched earlier.
#[derive(Debug, Clone)]
pub struct HistoryTable {
    scores: [[i32; SQUARE_COUNT]; SQUARE_COUNT],
}

impl HistoryTable {
    /// Creates a new history table
    pub fn new() -> Self {
        Self {
            scores: [[0; SQUARE_COUNT]; SQUARE_COUNT],
        }
    }

    /// Records a move that caused a beta cutoff
    ///
    /// The score increment is depth squared to give more weight to
    /// moves that cause cutoffs at deeper search depth as they are
    /// more significant.
    ///
    /// # Arguments
    /// * `mv` - The move that caused the cutoff
    /// * `depth` - The depth at which the cutoff occurred
    pub fn record_cutoff(&mut self, mv: &Move, depth: u8) {
        let from = mv.from as usize;
        let to = mv.to as usize;

        let increment = (depth as i32) * (depth as i32);

        self.scores[from][to] = self.scores[from][to].saturating_add(increment);
    }

    /// Gets the history score for a move
    ///
    /// # Arguments
    /// * `mv` - The move to get the score for
    ///
    /// # Returns
    /// The history score
    pub fn get_score(&self, mv: &Move) -> i32 {
        let from = mv.from as usize;
        let to = mv.to as usize;

        self.scores[from][to]
    }

    /// Ages all history scores by dividing by 2
    pub fn age(&mut self) {
        for from in 0..SQUARE_COUNT {
            for to in 0..SQUARE_COUNT {
                self.scores[from][to] /= 2;
            }
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
        assert_eq!(history.get_score(&mv), 0);
    }

    #[test]
    fn test_record_cutoff_increases_score() {
        let mut history = HistoryTable::new();
        let mv = create_test_move(12, 28);

        history.record_cutoff(&mv, 5);
        assert_eq!(history.get_score(&mv), 25);

        history.record_cutoff(&mv, 3);
        assert_eq!(history.get_score(&mv), 34);
    }

    #[test]
    fn test_deeper_cutoffs_score_higher() {
        let mut history = HistoryTable::new();
        let mv1 = create_test_move(12, 28);
        let mv2 = create_test_move(6, 21);

        history.record_cutoff(&mv1, 10);
        history.record_cutoff(&mv2, 5);

        assert!(history.get_score(&mv1) > history.get_score(&mv2));
    }

    #[test]
    fn test_different_moves_have_independent_scores() {
        let mut history = HistoryTable::new();
        let mv1 = create_test_move(12, 28);
        let mv2 = create_test_move(6, 21);

        history.record_cutoff(&mv1, 5);

        assert_eq!(history.get_score(&mv1), 25);
        assert_eq!(history.get_score(&mv2), 0);
    }

    #[test]
    fn test_age_reduces_scores() {
        let mut history = HistoryTable::new();
        let mv = create_test_move(12, 28);

        history.record_cutoff(&mv, 10);
        assert_eq!(history.get_score(&mv), 100);

        history.age();
        assert_eq!(history.get_score(&mv), 50);

        history.age();
        assert_eq!(history.get_score(&mv), 25);
    }
}
