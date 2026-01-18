/// Manages position history for repetition detection
#[derive(Debug, Clone)]
pub struct PositionHistory {
    hashes: Vec<u64>,
}

impl PositionHistory {
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

    /// Checks if a position has been repeated
    ///
    /// # Arguments
    /// * `current_hash` - The zobrist hash to check for repetition
    ///
    /// # Returns
    /// `true` if three-fold repetition is detected and `false` otherwise
    pub fn is_repetition(&self, current_hash: u64) -> bool {
        let mut count = 0;

        for &hash in self.hashes.iter().rev() {
            if hash == current_hash {
                count += 1;
                if count >= 2 {
                    return true;
                }
            }
        }

        false
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

impl Default for PositionHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_history() {
        let history = PositionHistory::new();

        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_push_and_len() {
        let mut history = PositionHistory::new();

        history.push(12345);
        assert_eq!(history.len(), 1);

        history.push(67890);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_pop() {
        let mut history = PositionHistory::new();

        history.push(12345);
        history.push(67890);

        history.pop();
        assert_eq!(history.len(), 1);

        history.pop();
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_pop_empty() {
        let mut history = PositionHistory::new();

        // Should not panic
        history.pop();
        assert!(history.is_empty());
    }

    #[test]
    fn test_is_repetition_no_repeat() {
        let mut history = PositionHistory::new();

        history.push(12345);
        history.push(67890);

        assert!(!history.is_repetition(11111));
    }

    #[test]
    fn test_is_repetition_three_fold() {
        let mut history = PositionHistory::new();

        history.push(12345);
        history.push(67890);
        history.push(12345);

        assert!(history.is_repetition(12345));
    }

    #[test]
    fn test_default_trait() {
        let history = PositionHistory::default();

        assert!(history.is_empty());
    }

    #[test]
    fn test_large_history() {
        let mut history = PositionHistory::new();

        // Add many positions
        for i in 0..500 {
            history.push(i);
        }

        assert_eq!(history.len(), 500);
        assert!(!history.is_repetition(1));

        // Add some repetitions
        history.push(100);
        history.push(100);

        assert!(history.is_repetition(100));
    }
}
