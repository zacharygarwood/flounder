use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::OnceLock;

use crate::bitboard::{BitboardIterator, SQUARES};
use crate::board::Board;
use crate::pieces::{Color, ColorIterator, Piece, PieceIterator, COLOR_COUNT, PIECE_COUNT};
use crate::square::Square;

const CASTLE_RIGHTS_COUNT: usize = 2; // King side and Queen side

/// King-side castling index into the castling key table.
pub const CASTLE_KING_SIDE: usize = 0;
/// Queen-side castling index into the castling key table.
pub const CASTLE_QUEEN_SIDE: usize = 1;

static ZOBRIST: OnceLock<ZobristTable> = OnceLock::new();

/// The single shared Zobrist key table.
///
/// Boards maintain their hash incrementally as moves are made, so every board
/// must hash against the same keys. Keys are randomized once per process.
pub fn zobrist() -> &'static ZobristTable {
    ZOBRIST.get_or_init(ZobristTable::new)
}

pub struct ZobristTable {
    table_keys: [[[u64; SQUARES as usize]; PIECE_COUNT]; COLOR_COUNT],
    white_to_move_key: u64,
    castling_right_keys: [[u64; CASTLE_RIGHTS_COUNT]; COLOR_COUNT],
    en_passant_target_key: [u64; SQUARES as usize],
}

impl ZobristTable {
    pub fn new() -> Self {
        // A fixed seed keeps keys identical across runs, so node counts are
        // reproducible for benchmarking. Only relative distribution matters.
        let mut rng = StdRng::seed_from_u64(0x9E3779B97F4A7C15);
        let mut table_keys = [[[0; SQUARES as usize]; PIECE_COUNT]; COLOR_COUNT];
        let mut castling_right_keys = [[0; CASTLE_RIGHTS_COUNT]; COLOR_COUNT];
        let mut en_passant_target_key = [0; SQUARES as usize];
        let white_to_move_key = rng.gen();

        for color_layer in &mut table_keys {
            for piece_layer in color_layer {
                for square in piece_layer {
                    *square = rng.gen();
                }
            }
        }

        for color_layer in &mut castling_right_keys {
            for right in color_layer {
                *right = rng.gen();
            }
        }

        for square in 0..SQUARES {
            en_passant_target_key[square as usize] = rng.gen();
        }

        Self {
            table_keys,
            white_to_move_key,
            castling_right_keys,
            en_passant_target_key,
        }
    }

    /// Key for a piece of `color` and `piece` type standing on `square`.
    #[inline]
    pub fn piece_key(&self, color: Color, piece: Piece, square: Square) -> u64 {
        self.table_keys[color.index()][piece.index()][square as usize]
    }

    /// Key for one castling right (`CASTLE_KING_SIDE` or `CASTLE_QUEEN_SIDE`).
    #[inline]
    pub fn castle_key(&self, color: Color, side: usize) -> u64 {
        self.castling_right_keys[color.index()][side]
    }

    /// Key for an en passant target on `square`.
    #[inline]
    pub fn ep_key(&self, square: Square) -> u64 {
        self.en_passant_target_key[square as usize]
    }

    /// Key toggled whenever the side to move changes.
    #[inline]
    pub fn side_key(&self) -> u64 {
        self.white_to_move_key
    }

    pub fn hash(&self, board: &Board) -> u64 {
        let mut hash: u64 = 0;

        let color_iter = ColorIterator::new();
        let piece_iter = PieceIterator::new();

        // Hash pieces
        for color in color_iter {
            for piece in piece_iter {
                let pieces = board.bb(color, piece);
                let bb_iter = BitboardIterator::new(pieces);
                for square in bb_iter {
                    hash ^= self.table_keys[color.index()][piece.index()][square as usize];
                }
            }
        }

        // Hash castling rights
        for color in color_iter {
            let (king_side, queen_side) = board.castling_ability(color);

            if king_side {
                hash ^= self.castling_right_keys[color.index()][0];
            }

            if queen_side {
                hash ^= self.castling_right_keys[color.index()][1];
            }
        }

        // Hash en passant target
        if let Some(square) = board.en_passant_target {
            hash ^= self.en_passant_target_key[square as usize];
        }

        // Hash active color
        if board.active_color == Color::White {
            hash ^= self.white_to_move_key;
        }

        hash
    }
}

#[cfg(test)]
mod tests {
    use crate::board::Board;
    use crate::zobrist::ZobristTable;

    #[test]
    fn same_positions_have_same_hash() {
        let zobrist = ZobristTable::new();

        let pos1 = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let pos2 = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        assert_eq!(zobrist.hash(&pos1), zobrist.hash(&pos2));
    }

    #[test]
    fn different_positions_have_different_hash() {
        let zobrist = ZobristTable::new();

        let pos = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let pos_different = Board::new("pnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

        assert_ne!(zobrist.hash(&pos), zobrist.hash(&pos_different));
    }

    #[test]
    fn different_castling_rights_have_different_hash() {
        let zobrist = ZobristTable::new();

        let pos = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let pos_no_castling = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1");

        assert_ne!(zobrist.hash(&pos), zobrist.hash(&pos_no_castling));
    }

    #[test]
    fn different_en_passant_targets_have_different_hash() {
        let zobrist = ZobristTable::new();

        let pos = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let pos_with_en_passant =
            Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - e4 0 1");

        assert_ne!(zobrist.hash(&pos), zobrist.hash(&pos_with_en_passant));
    }

    #[test]
    fn different_colors_have_different_hash() {
        let zobrist = ZobristTable::new();

        let pos = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let pos_different_color =
            Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b - - 0 1");

        assert_ne!(zobrist.hash(&pos), zobrist.hash(&pos_different_color));
    }

    // Walks the move tree and checks that the hash maintained incrementally by
    // make_move stays equal to a full recomputation at every node. This covers
    // all move types, castling-right changes, and en passant set/clear.
    fn check_incremental(move_gen: &crate::move_gen::MoveGenerator, board: &Board, depth: u8) {
        assert_eq!(
            board.hash,
            super::zobrist().hash(board),
            "incremental hash diverged from recompute"
        );

        if depth == 0 {
            return;
        }

        for mv in move_gen.generate_moves(board) {
            let next = board.clone_with_move(&mv);
            check_incremental(move_gen, &next, depth - 1);
        }
    }

    #[test]
    fn incremental_hash_matches_recompute() {
        let move_gen = crate::move_gen::MoveGenerator::new();
        let positions = [
            // Start position: double pushes set/clear en passant.
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            // Kiwipete: castling both sides, captures, rook moves.
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            // Promotions and rook captures affecting castling rights.
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            // Position rich in en passant opportunities.
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        ];

        for fen in positions {
            let board = Board::new(fen);
            check_incremental(&move_gen, &board, 3);
        }
    }
}

