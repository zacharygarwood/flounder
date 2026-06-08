/// Manages position history for repetition detection
#[derive(Debug, Clone)]
pub struct RepetitionTable {
    hashes: Vec<u64>,
}

impl RepetitionTable {
    /// Creates a new empty position history
    pub fn new() -> Self {
        Self {
            hashes: Vec::with_capacity(256),
        }
    }

    /// Adds a position hash to the history
    ///
    /// # Arguments
    /// * `hash` - Zobrist hash of the position
    pub fn push(&mut self, hash: u64) {
        self.hashes.push(hash);
    }

    /// Removes the last position from history
    pub fn pop(&mut self) {
        self.hashes.pop();
    }

    /// Checks if a position has occurred before in the history.
    ///
    /// The history holds the game line plus the positions on the current search
    /// path, and never the position being queried, so a single match means this
    /// position is being reached for at least the second time. Treating that
    /// first repetition as a draw (rather than waiting for the full three-fold)
    /// lets a winning side recognize and avoid drifting into a repetition draw.
    ///
    /// # Arguments
    /// * `current_hash` - The zobrist hash to check for repetition
    ///
    /// # Returns
    /// `true` if the position has occurred before and `false` otherwise
    pub fn is_repetition(&self, current_hash: u64) -> bool {
        self.hashes.iter().any(|&hash| hash == current_hash)
    }

    /// Gets the number of positions in history
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// Checks if history is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

impl Default for RepetitionTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_history() {
        let history = RepetitionTable::new();

        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_push_and_len() {
        let mut history = RepetitionTable::new();

        history.push(12345);
        assert_eq!(history.len(), 1);

        history.push(67890);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_pop() {
        let mut history = RepetitionTable::new();

        history.push(12345);
        history.push(67890);

        history.pop();
        assert_eq!(history.len(), 1);

        history.pop();
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_pop_empty() {
        let mut history = RepetitionTable::new();

        // Should not panic
        history.pop();
        assert!(history.is_empty());
    }

    #[test]
    fn test_is_repetition_no_repeat() {
        let mut history = RepetitionTable::new();

        history.push(12345);
        history.push(67890);

        assert!(!history.is_repetition(11111));
    }

    #[test]
    fn test_is_repetition_three_fold() {
        let mut history = RepetitionTable::new();

        history.push(12345);
        history.push(67890);
        history.push(12345);

        assert!(history.is_repetition(12345));
    }

    #[test]
    fn test_default_trait() {
        let history = RepetitionTable::default();

        assert!(history.is_empty());
    }

    #[test]
    fn test_large_history() {
        let mut history = RepetitionTable::new();

        // Add many positions
        for i in 0..500 {
            history.push(i);
        }

        assert_eq!(history.len(), 500);
        assert!(!history.is_repetition(1000));

        assert!(history.is_repetition(100));
    }
}
