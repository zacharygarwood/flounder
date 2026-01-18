use rand::Rng;

use crate::bitboard::{BitboardIterator, SQUARES};
use crate::board::Board;
use crate::pieces::{Color, ColorIterator, PieceIterator, COLOR_COUNT, PIECE_COUNT};

const CASTLE_RIGHTS_COUNT: usize = 2; // King side and Queen side

pub struct ZobristTable {
    table_keys: [[[u64; SQUARES as usize]; PIECE_COUNT]; COLOR_COUNT],
    white_to_move_key: u64,
    castling_right_keys: [[u64; CASTLE_RIGHTS_COUNT]; COLOR_COUNT],
    en_passant_target_key: [u64; SQUARES as usize],
}

impl ZobristTable {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
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
}

