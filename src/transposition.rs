use crate::moves::Move;

/// Number of slots in the table (a power of two so the index is a mask of the
/// zobrist key). At ~24 bytes per slot this is roughly 100 MB.
const TT_ENTRIES: usize = 1 << 22;

pub struct TranspositionTable {
    table: Vec<Option<Entry>>,
    mask: usize,
    age: u8,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            table: allocate_table(),
            mask: TT_ENTRIES - 1,
            age: 0,
        }
    }

    /// Advances the table's age at the start of a new search so entries left
    /// over from previous searches can be overwritten regardless of depth.
    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    #[inline]
    fn index(&self, hash_key: u64) -> usize {
        hash_key as usize & self.mask
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
        let index = self.index(hash_key);

        // Replace when the slot is empty, holds an entry from an earlier search,
        // or the new result reached at least as deep (depth-preferred).
        let replace = match &self.table[index] {
            None => true,
            Some(existing) => existing.age != self.age || depth >= existing.depth,
        };

        if replace {
            self.table[index] = Some(Entry {
                hash_key,
                eval,
                static_eval,
                best_move,
                depth,
                bounds,
                age: self.age,
            });
        }
    }

    /// Starts loading `hash_key`'s slot into cache. The table is far larger
    /// than the cache, so a probe is usually a trip to main memory; issuing
    /// this as soon as a child's hash is known overlaps that trip with the
    /// work done before the child probes.
    #[inline]
    pub fn prefetch(&self, hash_key: u64) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
            let slot = &self.table[self.index(hash_key)] as *const Option<Entry> as *const i8;
            // SAFETY: prefetching is only a hint; it never faults or reads
            // into Rust-visible state, and the address is an allocated slot.
            unsafe { _mm_prefetch(slot, _MM_HINT_T0) };
        }
    }

    pub fn retrieve(&self, key: u64) -> Option<&Entry> {
        match &self.table[self.index(key)] {
            Some(entry) if entry.hash_key == key => Some(entry),
            _ => None,
        }
    }
}

/// Allocates the (empty) table, backed by 2 MB huge pages where the OS allows.
///
/// With 4 KB pages, a random probe into a ~100 MB table nearly always misses
/// the TLB as well as the cache; huge pages cut the page count 512-fold so the
/// translations stay cached. The memory is only touched (and so faulted in)
/// after the advice is given, so the kernel can back it with huge pages from
/// the start rather than collapsing small ones later.
fn allocate_table() -> Vec<Option<Entry>> {
    let mut table: Vec<Option<Entry>> = Vec::with_capacity(TT_ENTRIES);
    #[cfg(target_os = "linux")]
    advise_huge_pages(table.as_mut_ptr() as usize, TT_ENTRIES * std::mem::size_of::<Option<Entry>>());
    table.resize(TT_ENTRIES, None);
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
    use crate::transposition::{TranspositionTable, Bounds};
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
    
}