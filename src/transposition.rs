use crate::moves::Move;

/// Number of buckets (a power of two so the index is a mask of the zobrist
/// key). At 4 slots of ~24 bytes per bucket this is roughly 100 MB.
const TT_BUCKETS: usize = 1 << 20;

/// Positions sharing a bucket. With several slots per index, a deep entry is
/// not evicted just because a shallow (e.g. quiescence) result for another
/// position landed on the same index.
const SLOTS_PER_BUCKET: usize = 4;

type Bucket = [Option<Entry>; SLOTS_PER_BUCKET];

pub struct TranspositionTable {
    table: Vec<Bucket>,
    mask: usize,
    age: u8,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            table: allocate_table(),
            mask: TT_BUCKETS - 1,
            age: 0,
        }
    }

    /// Advances the table's age at the start of a new search so entries left
    /// over from previous searches can be overwritten regardless of depth.
    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    pub fn store(
        &mut self,
        hash_key: u64,
        eval: i32,
        static_eval: Option<i16>,
        best_move: Option<Move>,
        depth: u8,
        bounds: Bounds,
    ) {
        let age = self.age;
        let bucket = &mut self.table[hash_key as usize & self.mask];

        // The same position is only overwritten by a result from a newer
        // search or at least as deep (depth-preferred). A new position takes
        // an empty slot, else evicts the least valuable entry: one from an
        // earlier search first, then the shallowest.
        let slot = match bucket.iter().position(|e| e.is_some_and(|e| e.hash_key == hash_key)) {
            Some(same) => {
                let existing = bucket[same].unwrap();
                (existing.age != age || depth >= existing.depth).then_some(same)
            }
            None => bucket
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| match e {
                    None => (0, 0),
                    Some(e) => (1 + (e.age == age) as u8, e.depth),
                })
                .map(|(i, _)| i),
        };

        if let Some(slot) = slot {
            bucket[slot] = Some(Entry {
                hash_key,
                eval,
                static_eval,
                best_move,
                depth,
                bounds,
                age,
            });
        }
    }

    /// Starts loading `hash_key`'s bucket into cache. The table is far
    /// larger than the cache, so a probe is usually a trip to main memory;
    /// issuing this as soon as a child's hash is known overlaps that trip with
    /// the work done before the child probes.
    #[inline]
    pub fn prefetch(&self, hash_key: u64) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
            let bucket = &self.table[hash_key as usize & self.mask] as *const Bucket as *const i8;
            // A bucket is wider than a cache line, so it can straddle two.
            // SAFETY: prefetching is only a hint; it never faults or reads
            // into Rust-visible state, and both addresses are within or just
            // past an allocated bucket.
            unsafe {
                _mm_prefetch(bucket, _MM_HINT_T0);
                _mm_prefetch(bucket.wrapping_add(std::mem::size_of::<Bucket>() - 1), _MM_HINT_T0);
            }
        }
    }

    pub fn retrieve(&self, key: u64) -> Option<&Entry> {
        self.table[key as usize & self.mask]
            .iter()
            .flatten()
            .find(|entry| entry.hash_key == key)
    }
}

/// Allocates the (empty) table, backed by 2 MB huge pages where the OS allows.
///
/// With 4 KB pages, a random probe into a ~100 MB table nearly always misses
/// the TLB as well as the cache; huge pages cut the page count 512-fold so the
/// translations stay cached. The memory is only touched (and so faulted in)
/// after the advice is given, so the kernel can back it with huge pages from
/// the start rather than collapsing small ones later.
fn allocate_table() -> Vec<Bucket> {
    let mut table: Vec<Bucket> = Vec::with_capacity(TT_BUCKETS);
    #[cfg(target_os = "linux")]
    advise_huge_pages(table.as_mut_ptr() as usize, TT_BUCKETS * std::mem::size_of::<Bucket>());
    table.resize(TT_BUCKETS, [None; SLOTS_PER_BUCKET]);
    table
}

/// Asks for huge pages over the 2 MB-aligned interior of `[start, start+len)`.
/// Only advice: if transparent huge pages are disabled it fails harmlessly
/// and the table simply uses normal pages.
#[cfg(target_os = "linux")]
fn advise_huge_pages(start: usize, len: usize) {
    const HUGE_PAGE: usize = 2 * 1024 * 1024;
    let aligned_start = start.next_multiple_of(HUGE_PAGE);
    let aligned_end = (start + len) / HUGE_PAGE * HUGE_PAGE;
    if aligned_end > aligned_start {
        // SAFETY: the range lies within the table's own allocation, and
        // MADV_HUGEPAGE does not change its contents or validity.
        unsafe {
            libc::madvise(aligned_start as *mut libc::c_void, aligned_end - aligned_start, libc::MADV_HUGEPAGE);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Entry {
    pub hash_key: u64,
    pub eval: i32,
    /// The node's static evaluation, so a later visit can skip evaluating.
    /// `None` when the node was in check or the eval was not computed.
    pub static_eval: Option<i16>,
    pub best_move: Option<Move>,
    pub depth: u8,
    pub bounds: Bounds,
    age: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Bounds {
    Exact,
    Lower,
    Upper,
}

#[cfg(test)]
mod tests {
    use crate::transposition::{TranspositionTable, Bounds, SLOTS_PER_BUCKET};
    use crate::zobrist::ZobristTable;
    use crate::pieces::Piece;
    use crate::moves::{Move, MoveType};
    use crate::board::Board;

    #[test]
    fn retrieve_position_in_table() {
        let mut tt = TranspositionTable::new();
        let zobrist = ZobristTable::new();
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        let stored_eval = 150;
        let stored_mv = Move::new(8, 16, Piece::Pawn, MoveType::Quiet);
        let stored_depth = 6;
        let stored_bounds = Bounds::Exact;

        tt.store(zobrist.hash(&board), stored_eval, None, Some(stored_mv), stored_depth, stored_bounds);

        let entry = tt.retrieve(zobrist.hash(&board));

        assert_eq!(stored_eval, entry.unwrap().eval);
        assert_eq!(stored_mv, entry.unwrap().best_move.unwrap());
        assert_eq!(stored_depth, entry.unwrap().depth);
        assert_eq!(stored_bounds, entry.unwrap().bounds);
    }

    #[test]
    fn retrieve_position_not_in_table() {
        let tt = TranspositionTable::new();
        let zobrist = ZobristTable::new();
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        let entry = tt.retrieve(zobrist.hash(&board));

        assert_eq!(entry, None);
    }

    #[test]
    fn update_entry_with_new_greater_depth() {
        let mut tt = TranspositionTable::new();
        let zobrist = ZobristTable::new();
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        let lower_depth_eval = 150;
        let lower_depth_mv = Move::new(8, 16, Piece::Pawn, MoveType::Quiet);
        let lower_depth_depth = 6;
        let lower_depth_bounds = Bounds::Exact;

        let greater_depth_eval = 100;
        let greater_depth_mv = Move::new(9, 17, Piece::Pawn, MoveType::Quiet);
        let greater_depth_depth = 7;
        let greater_depth_bounds = Bounds::Lower;

        // Store entry with lower depth first, then store entry with greater depth
        // lower depth should be replaced with greater depth
        tt.store(zobrist.hash(&board), lower_depth_eval, None, Some(lower_depth_mv), lower_depth_depth, lower_depth_bounds);
        tt.store(zobrist.hash(&board), greater_depth_eval, None, Some(greater_depth_mv), greater_depth_depth, greater_depth_bounds);

        let entry = tt.retrieve(zobrist.hash(&board));

        assert_eq!(greater_depth_eval, entry.unwrap().eval);
        assert_eq!(greater_depth_mv, entry.unwrap().best_move.unwrap());
        assert_eq!(greater_depth_depth, entry.unwrap().depth);
        assert_eq!(greater_depth_bounds, entry.unwrap().bounds);
    }

    #[test]
    fn keep_entry_with_old_greater_depth() {
        let mut tt = TranspositionTable::new();
        let zobrist = ZobristTable::new();
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        let lower_depth_eval = 150;
        let lower_depth_mv = Move::new(8, 16, Piece::Pawn, MoveType::Quiet);
        let lower_depth_depth = 6;
        let lower_depth_bounds = Bounds::Exact;

        let greater_depth_eval = 100;
        let greater_depth_mv = Move::new(9, 17, Piece::Pawn, MoveType::Quiet);
        let greater_depth_depth = 7;
        let greater_depth_bounds = Bounds::Lower;

        // Store entry with greater depth first, then store entry with lower depth
        // greater depth should not be replaced with lower depth
        tt.store(zobrist.hash(&board), greater_depth_eval, None, Some(greater_depth_mv), greater_depth_depth, greater_depth_bounds);
        tt.store(zobrist.hash(&board), lower_depth_eval, None, Some(lower_depth_mv), lower_depth_depth, lower_depth_bounds);

        let entry = tt.retrieve(zobrist.hash(&board));

        assert_eq!(greater_depth_eval, entry.unwrap().eval);
        assert_eq!(greater_depth_mv, entry.unwrap().best_move.unwrap());
        assert_eq!(greater_depth_depth, entry.unwrap().depth);
        assert_eq!(greater_depth_bounds, entry.unwrap().bounds);
    }
    

    /// Hash keys that share a bucket (same low bits) but are different
    /// positions.
    fn bucket_mates(count: u64) -> Vec<u64> {
        (1..=count).map(|i| 7 | (i << 40)).collect()
    }

    #[test]
    fn deep_entry_survives_shallow_entries_for_other_positions() {
        let mut tt = TranspositionTable::new();
        let keys = bucket_mates(SLOTS_PER_BUCKET as u64 + 3);

        tt.store(keys[0], 10, None, None, 12, Bounds::Exact);
        for &key in &keys[1..] {
            tt.store(key, 0, None, None, 0, Bounds::Upper);
        }

        assert_eq!(tt.retrieve(keys[0]).unwrap().depth, 12);
    }

    #[test]
    fn positions_sharing_a_bucket_are_kept_apart() {
        let mut tt = TranspositionTable::new();
        let keys = bucket_mates(2);

        tt.store(keys[0], 10, None, None, 3, Bounds::Exact);
        tt.store(keys[1], 20, None, None, 3, Bounds::Exact);

        assert_eq!(tt.retrieve(keys[0]).unwrap().eval, 10);
        assert_eq!(tt.retrieve(keys[1]).unwrap().eval, 20);
    }

    #[test]
    fn entries_from_an_earlier_search_are_evicted_first() {
        let mut tt = TranspositionTable::new();
        let keys = bucket_mates(SLOTS_PER_BUCKET as u64 + 1);

        tt.store(keys[0], 10, None, None, 20, Bounds::Exact);
        tt.new_search();
        for &key in &keys[1..] {
            tt.store(key, 0, None, None, 5, Bounds::Exact);
        }

        assert!(tt.retrieve(keys[0]).is_none());
        assert!(keys[1..].iter().all(|&key| tt.retrieve(key).is_some()));
    }
}
