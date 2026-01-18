use crate::moves::Move;

/// Maximum search depth supported
const MAX_DEPTH: usize = 64;

/// Number of killer moves to track per ply
const KILLERS_PER_PLY: usize = 2;

/// Manages killer moves for efficient move ordering
#[derive(Debug, Clone)]
pub struct KillerMoves {
    moves: [[Option<Move>; KILLERS_PER_PLY]; MAX_DEPTH],
}

impl KillerMoves {
    /// Creates a new empty killer move table
    pub fn new() -> Self {
        Self {
            moves: [[None; KILLERS_PER_PLY]; MAX_DEPTH],
        }
    }

    /// Stores a killer move at the specified ply
    ///
    /// Moves are stored in a FIFO manner. Duplicate moves at
    /// the same ply are not stored again.
    ///
    /// # Arguments
    /// * `mv` - The move to store
    /// * `ply` - The ply depth where this move caused a cutoff
    pub fn store(&mut self, mv: Move, ply: u8) {
        if !self.is_valid_ply(ply) {
            return;
        }

        let ply_idx = ply as usize;

        // Don't store if it's already the primary killer
        if self.moves[ply_idx][0] == Some(mv) {
            return;
        }

        self.shift_and_insert(ply_idx, mv);
    }

    /// Checks if a move is a killer move at the specified ply
    ///
    /// # Arguments
    /// * `mv` - The move to check
    /// * `ply` - The ply depth to check at
    ///
    /// # Returns
    /// `true` if the move is a killer at this ply, `false` otherwise
    pub fn is_killer(&self, mv: &Move, ply: u8) -> bool {
        if !self.is_valid_ply(ply) {
            return false;
        }

        let ply_idx = ply as usize;
        self.moves[ply_idx].contains(&Some(*mv))
    }

    /// Gets all killer moves at a specific ply
    ///
    /// # Arguments
    /// * `ply` - The ply depth to retrieve killers from
    ///
    /// # Returns
    /// A slice of optional moves
    #[allow(dead_code)]
    pub fn get_killers(&self, ply: u8) -> &[Option<Move>] {
        if !self.is_valid_ply(ply) {
            return &[];
        }

        &self.moves[ply as usize]
    }

    /// Clears all killer moves
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.moves = [[None; KILLERS_PER_PLY]; MAX_DEPTH];
    }

    /// Clears killer moves at a specific ply
    ///
    /// # Arguments
    /// * `ply` - The ply depth to clear
    #[allow(dead_code)]
    pub fn clear_ply(&mut self, ply: u8) {
        if self.is_valid_ply(ply) {
            self.moves[ply as usize] = [None; KILLERS_PER_PLY];
        }
    }

    /// Validates that a ply is within supported bounds
    fn is_valid_ply(&self, ply: u8) -> bool {
        (ply as usize) < MAX_DEPTH
    }

    /// Shifts existing killers and inserts a new one
    fn shift_and_insert(&mut self, ply_idx: usize, mv: Move) {
        for i in (1..KILLERS_PER_PLY).rev() {
            self.moves[ply_idx][i] = self.moves[ply_idx][i - 1];
        }

        self.moves[ply_idx][0] = Some(mv);
    }
}

impl Default for KillerMoves {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::MoveType;
    use crate::pieces::Piece;

    #[test]
    fn test_new_creates_empty_table() {
        let killers = KillerMoves::new();

        for ply in 0..10 {
            assert!(!killers.is_killer(&create_test_move(8, 16), ply));
        }
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);

        killers.store(move1, 0);

        assert!(killers.is_killer(&move1, 0));
        assert!(!killers.is_killer(&move1, 1)); // Different ply
    }

    #[test]
    fn test_store_multiple_moves() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);
        let move2 = create_test_move(9, 17);

        killers.store(move1, 0);
        killers.store(move2, 0);

        assert!(killers.is_killer(&move1, 0));
        assert!(killers.is_killer(&move2, 0));
    }

    #[test]
    fn test_fifo_replacement() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);
        let move2 = create_test_move(9, 17);
        let move3 = create_test_move(10, 18);

        killers.store(move1, 0);
        killers.store(move2, 0);
        killers.store(move3, 0);

        // move3 should be primary, move2 secondary, move1 should be gone
        assert!(killers.is_killer(&move3, 0));
        assert!(killers.is_killer(&move2, 0));
        assert!(!killers.is_killer(&move1, 0));
    }

    #[test]
    fn test_no_duplicate_storage() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);
        let move2 = create_test_move(9, 17);

        killers.store(move1, 0);
        killers.store(move2, 0);
        killers.store(move1, 0); // Try to store move1 again

        let stored_killers = killers.get_killers(0);

        // move1 should still be in the first slot (not duplicated)
        assert_eq!(stored_killers[0], Some(move1));
        assert_eq!(stored_killers[1], Some(move2));
    }

    #[test]
    fn test_different_plies() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);
        let move2 = create_test_move(9, 17);

        killers.store(move1, 0);
        killers.store(move2, 1);

        assert!(killers.is_killer(&move1, 0));
        assert!(!killers.is_killer(&move1, 1));
        assert!(!killers.is_killer(&move2, 0));
        assert!(killers.is_killer(&move2, 1));
    }

    #[test]
    fn test_clear() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);

        killers.store(move1, 0);
        assert!(killers.is_killer(&move1, 0));

        killers.clear();
        assert!(!killers.is_killer(&move1, 0));
    }

    #[test]
    fn test_clear_specific_ply() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);
        let move2 = create_test_move(9, 17);

        killers.store(move1, 0);
        killers.store(move2, 1);

        killers.clear_ply(0);

        assert!(!killers.is_killer(&move1, 0));
        assert!(killers.is_killer(&move2, 1)); // Ply 1 should be unaffected
    }

    #[test]
    fn test_invalid_ply_bounds() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);

        // Should handle gracefully without panicking
        killers.store(move1, 255);
        assert!(!killers.is_killer(&move1, 255));

        killers.clear_ply(255);
        let empty_slice = killers.get_killers(255);
        assert_eq!(empty_slice.len(), 0);
    }

    #[test]
    fn test_get_killers() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);
        let move2 = create_test_move(9, 17);

        killers.store(move1, 0);
        killers.store(move2, 0);

        let stored = killers.get_killers(0);

        assert_eq!(stored.len(), KILLERS_PER_PLY);
        assert_eq!(stored[0], Some(move2)); // Most recent
        assert_eq!(stored[1], Some(move1)); // Second most recent
    }

    #[test]
    fn test_max_depth_boundary() {
        let mut killers = KillerMoves::new();
        let move1 = create_test_move(8, 16);

        // Test at maximum valid ply
        let max_ply = (MAX_DEPTH - 1) as u8;
        killers.store(move1, max_ply);
        assert!(killers.is_killer(&move1, max_ply));

        // Test beyond maximum
        let invalid_ply = MAX_DEPTH as u8;
        killers.store(move1, invalid_ply);
        assert!(!killers.is_killer(&move1, invalid_ply));
    }

    #[test]
    fn test_default_trait() {
        let killers = KillerMoves::default();
        let move1 = create_test_move(8, 16);

        assert!(!killers.is_killer(&move1, 0));
    }

    // Helper function to create test moves
    fn create_test_move(from: u8, to: u8) -> Move {
        Move::new(from, to, Piece::Pawn, MoveType::Quiet)
    }
}
