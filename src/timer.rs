use crate::moves::Move;
use std::time::{Duration, Instant};

/// Manages search timing and statistics
#[derive(Debug, Clone)]
pub struct SearchTimer {
    start_time: Option<Instant>,
    time_limit: Option<Duration>,
    nodes_searched: u64,
}

impl SearchTimer {
    /// Creates a new search timer
    pub fn new() -> Self {
        Self {
            start_time: None,
            time_limit: None,
            nodes_searched: 0,
        }
    }

    /// Starts a new search with an optional time limit
    ///
    /// # Arguments
    /// * `time_limit` - Optional max duration for the search
    pub fn start(&mut self, time_limit: Option<Duration>) {
        self.start_time = Some(Instant::now());
        self.time_limit = time_limit;
        self.nodes_searched = 0;
    }

    /// Resets the timer without changing the time limit
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.start_time = Some(Instant::now());
        self.nodes_searched = 0;
    }

    /// Increments the node counter
    #[inline]
    pub fn increment_nodes(&mut self) {
        self.nodes_searched += 1;
    }

    /// Adds multiple nodes to the counter
    ///
    /// # Arguments
    /// * `count` - Number of nodes to add
    #[inline]
    #[allow(dead_code)]
    pub fn add_nodes(&mut self, count: u64) {
        self.nodes_searched += count;
    }

    /// Checks if the saerch should stop due to time limit
    ///
    /// # Returns
    /// `true` if time limit exceeded, `false` otherwise
    pub fn should_stop(&self) -> bool {
        if let (Some(start), Some(limit)) = (self.start_time, self.time_limit) {
            start.elapsed() >= limit
        } else {
            false
        }
    }

    /// Gets the number of nodes searched
    #[allow(dead_code)]
    pub fn nodes(&self) -> u64 {
        self.nodes_searched
    }

    /// Gets the elapsed time in milliseconds
    ///
    /// # Returns
    /// Elapsed milliseconds or 0 if search hasn't started
    pub fn elapsed_ms(&self) -> u128 {
        self.start_time
            .map(|start| start.elapsed().as_millis())
            .unwrap_or(0)
    }

    /// Gets the elapsed time as a Duration
    #[allow(dead_code)]
    pub fn elapsed(&self) -> Duration {
        self.start_time
            .map(|start| start.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// Calculates the nodes per second
    ///
    /// # Returns
    /// Nodes per second or 0 if no time has elapsed
    pub fn nps(&self) -> u128 {
        let elapsed = self.elapsed_ms().max(1);
        (self.nodes_searched as u128 * 1000) / elapsed
    }

    /// Gets a formatted string of search statistics
    ///
    /// # Returns
    /// String in format "nodes: X, time: Yms, nps: Z"
    #[allow(dead_code)]
    pub fn stats_string(&self) -> String {
        format!(
            "nodes: {}, time: {}ms, nps: {}",
            self.nodes_searched,
            self.elapsed_ms(),
            self.nps(),
        )
    }

    /// Prints UCI-formatted search information
    ///
    /// # Arguments
    /// * `depth` - Current search depth
    /// * `score` - Current best score (in centipawns)
    /// * `best_move` - Current best move
    pub fn print_info(&self, depth: u8, score: i32, best_move: Option<Move>) {
        print!(
            "info depth {} score cp {} nodes {} time {} nps {}",
            depth,
            score,
            self.nodes_searched,
            self.elapsed_ms(),
            self.nps()
        );

        if let Some(mv) = best_move {
            print!(" pv {}", mv.to_algebraic());
        }
        println!();
    }

    /// Checks if a search has started
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    }

    /// Gets the time limit if one is set
    #[allow(dead_code)]
    pub fn time_limit(&self) -> Option<Duration> {
        self.time_limit
    }

    /// Gets the remaining time in the search
    ///
    /// # Returns
    /// Remaining duration or None if no time limit is set
    #[allow(dead_code)]
    pub fn time_remaining(&self) -> Option<Duration> {
        if let (Some(start), Some(limit)) = (self.start_time, self.time_limit) {
            let elapsed = start.elapsed();
            if elapsed < limit {
                Some(limit - elapsed)
            } else {
                Some(Duration::ZERO)
            }
        } else {
            None
        }
    }
}

impl Default for SearchTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_creates_inactive_timer() {
        let timer = SearchTimer::new();

        assert!(!timer.is_running());
        assert_eq!(timer.nodes(), 0);
        assert_eq!(timer.elapsed_ms(), 0);
    }

    #[test]
    fn test_start_activates_timer() {
        let mut timer = SearchTimer::new();

        timer.start(None);

        assert!(timer.is_running());
        assert_eq!(timer.nodes(), 0);
    }

    #[test]
    fn test_start_with_time_limit() {
        let mut timer = SearchTimer::new();
        let limit = Duration::from_secs(5);

        timer.start(Some(limit));

        assert_eq!(timer.time_limit(), Some(limit));
    }

    #[test]
    fn test_increment_nodes() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        timer.increment_nodes();
        assert_eq!(timer.nodes(), 1);

        timer.increment_nodes();
        assert_eq!(timer.nodes(), 2);
    }

    #[test]
    fn test_add_nodes() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        timer.add_nodes(10);
        assert_eq!(timer.nodes(), 10);

        timer.add_nodes(5);
        assert_eq!(timer.nodes(), 15);
    }

    #[test]
    fn test_elapsed_time_increases() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        thread::sleep(Duration::from_millis(10));

        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10, "Expected at least 10ms, got {}ms", elapsed);
    }

    #[test]
    fn test_should_stop_no_limit() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        thread::sleep(Duration::from_millis(50));

        assert!(!timer.should_stop());
    }

    #[test]
    fn test_should_stop_with_limit() {
        let mut timer = SearchTimer::new();
        timer.start(Some(Duration::from_millis(10)));

        thread::sleep(Duration::from_millis(20));

        assert!(timer.should_stop());
    }

    #[test]
    fn test_should_not_stop_under_limit() {
        let mut timer = SearchTimer::new();
        timer.start(Some(Duration::from_millis(100)));

        thread::sleep(Duration::from_millis(10));

        assert!(!timer.should_stop());
    }

    #[test]
    fn test_nps_calculation() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        timer.add_nodes(1000);
        thread::sleep(Duration::from_millis(10));

        let nps = timer.nps();
        // Should be approximately 100,000 nps (1000 nodes / 0.01 seconds)
        assert!(nps > 50_000, "NPS too low: {}", nps);
    }

    #[test]
    fn test_reset() {
        let mut timer = SearchTimer::new();
        timer.start(Some(Duration::from_secs(10)));

        timer.add_nodes(100);
        thread::sleep(Duration::from_millis(10));

        timer.reset();

        assert_eq!(timer.nodes(), 0);
        assert!(timer.elapsed_ms() < 5); // Should be near zero
        assert_eq!(timer.time_limit(), Some(Duration::from_secs(10))); // Limit preserved
    }

    #[test]
    fn test_stats_string() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        timer.add_nodes(1000);
        thread::sleep(Duration::from_millis(10));

        let stats = timer.stats_string();

        assert!(stats.contains("nodes: 1000"));
        assert!(stats.contains("time:"));
        assert!(stats.contains("nps:"));
    }

    #[test]
    fn test_time_remaining_no_limit() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        assert_eq!(timer.time_remaining(), None);
    }

    #[test]
    fn test_time_remaining_with_limit() {
        let mut timer = SearchTimer::new();
        let limit = Duration::from_millis(100);
        timer.start(Some(limit));

        thread::sleep(Duration::from_millis(20));

        let remaining = timer.time_remaining().unwrap();
        assert!(remaining <= Duration::from_millis(80));
        assert!(remaining >= Duration::from_millis(70));
    }

    #[test]
    fn test_time_remaining_expired() {
        let mut timer = SearchTimer::new();
        timer.start(Some(Duration::from_millis(10)));

        thread::sleep(Duration::from_millis(20));

        let remaining = timer.time_remaining().unwrap();
        assert_eq!(remaining, Duration::ZERO);
    }

    #[test]
    fn test_multiple_starts() {
        let mut timer = SearchTimer::new();

        timer.start(Some(Duration::from_secs(10)));
        timer.add_nodes(100);

        timer.start(Some(Duration::from_secs(5))); // New search

        assert_eq!(timer.nodes(), 0); // Should be reset
        assert_eq!(timer.time_limit(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_default_trait() {
        let timer = SearchTimer::default();

        assert!(!timer.is_running());
        assert_eq!(timer.nodes(), 0);
    }

    #[test]
    fn test_elapsed_duration() {
        let mut timer = SearchTimer::new();
        timer.start(None);

        thread::sleep(Duration::from_millis(20));

        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::from_millis(20));
    }
}
