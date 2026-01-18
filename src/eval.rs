use crate::bitboard::{BitboardIterator, SQUARES};
use crate::board::Board;
use crate::pieces::{Color, Piece, PIECE_COUNT};

type PST = [i32; SQUARES as usize];

const OPENING_TABLES: [PST; PIECE_COUNT] = [
    // Pawn (82 + positional)
    [
        82, 82, 82, 82, 82, 82, 82, 82, 180, 216, 143, 177, 150, 208, 116, 71, 76, 89, 108, 113,
        147, 138, 107, 62, 68, 95, 88, 103, 105, 94, 99, 59, 55, 80, 77, 94, 99, 88, 92, 57, 56,
        78, 78, 72, 85, 85, 115, 70, 47, 81, 62, 59, 67, 106, 120, 60, 82, 82, 82, 82, 82, 82, 82,
        82,
    ],
    // Knight (337 + positional)
    [
        170, 248, 303, 288, 398, 240, 322, 230, 264, 296, 409, 373, 360, 399, 344, 320, 290, 397,
        374, 402, 421, 466, 410, 381, 328, 354, 356, 390, 374, 406, 355, 359, 324, 341, 353, 350,
        365, 356, 358, 329, 314, 328, 349, 347, 356, 354, 362, 321, 308, 284, 325, 334, 336, 355,
        323, 318, 232, 316, 279, 304, 320, 309, 318, 314,
    ],
    // Bishop (365 + positional)
    [
        336, 369, 283, 328, 340, 323, 372, 357, 339, 381, 347, 352, 395, 424, 383, 318, 349, 402,
        408, 405, 400, 415, 402, 363, 361, 370, 384, 415, 402, 402, 372, 363, 359, 378, 378, 391,
        399, 377, 375, 369, 365, 380, 380, 380, 379, 392, 383, 375, 369, 380, 381, 365, 372, 386,
        398, 366, 332, 362, 351, 344, 352, 353, 326, 344,
    ],
    // Rook (477 + positional)
    [
        509, 519, 509, 528, 540, 486, 508, 520, 504, 509, 535, 539, 557, 544, 503, 521, 472, 496,
        503, 513, 494, 522, 538, 493, 453, 466, 484, 503, 501, 512, 469, 457, 441, 451, 465, 476,
        486, 470, 483, 454, 432, 452, 461, 460, 480, 477, 472, 444, 433, 461, 457, 468, 476, 488,
        471, 406, 458, 464, 478, 494, 493, 484, 440, 451,
    ],
    // Queen (1025 + positional)
    [
        997, 1025, 1054, 1037, 1084, 1069, 1068, 1070, 1001, 986, 1020, 1026, 1009, 1082, 1053,
        1079, 1012, 1008, 1032, 1033, 1054, 1081, 1072, 1082, 998, 998, 1009, 1009, 1024, 1042,
        1023, 1026, 1016, 999, 1016, 1015, 1023, 1021, 1028, 1022, 1011, 1027, 1014, 1023, 1020,
        1027, 1039, 1030, 990, 1017, 1036, 1027, 1033, 1040, 1022, 1026, 1024, 1007, 1016, 1035,
        1010, 1000, 994, 975,
    ],
    // King (0 + positional)
    [
        -65, 23, 16, -15, -56, -34, 2, 13, 29, -1, -20, -7, -8, -4, -38, -29, -9, 24, 2, -16, -20,
        6, 22, -22, -17, -20, -12, -27, -30, -25, -14, -36, -49, -1, -27, -39, -46, -44, -33, -51,
        -14, -14, -22, -46, -44, -30, -15, -27, 1, 7, -8, -64, -43, -16, 9, 8, -15, 36, 12, -54, 8,
        -28, 24, 14,
    ],
];

const ENDGAME_TABLES: [PST; PIECE_COUNT] = [
    // Pawn (94 + positional)
    [
        94, 94, 94, 94, 94, 94, 94, 94, 272, 267, 252, 228, 241, 226, 259, 281, 188, 194, 179, 161,
        150, 147, 176, 178, 126, 118, 107, 99, 92, 98, 111, 111, 107, 103, 91, 87, 87, 86, 97, 93,
        98, 101, 88, 95, 94, 89, 93, 86, 107, 102, 102, 104, 107, 94, 96, 87, 94, 94, 94, 94, 94,
        94, 94, 94,
    ],
    // Knight (281 + positional)
    [
        223, 243, 268, 253, 250, 254, 218, 182, 256, 273, 256, 279, 272, 256, 257, 229, 257, 261,
        291, 290, 280, 272, 262, 240, 264, 284, 303, 303, 303, 292, 289, 263, 263, 275, 297, 306,
        297, 298, 285, 263, 258, 278, 280, 296, 291, 278, 261, 259, 239, 261, 271, 276, 279, 261,
        258, 237, 252, 230, 258, 266, 259, 263, 231, 217,
    ],
    // Bishop (297 + positional)
    [
        283, 276, 286, 289, 290, 288, 280, 273, 289, 293, 304, 285, 294, 284, 293, 283, 299, 289,
        297, 296, 295, 303, 297, 301, 294, 306, 309, 306, 311, 307, 300, 299, 291, 300, 310, 316,
        304, 307, 294, 288, 285, 294, 305, 307, 310, 300, 290, 282, 283, 279, 290, 296, 301, 288,
        282, 270, 274, 288, 274, 292, 288, 281, 292, 280,
    ],
    // Rook (512 + positional)
    [
        525, 522, 530, 527, 524, 524, 520, 517, 523, 525, 525, 523, 509, 515, 520, 515, 519, 519,
        519, 517, 516, 509, 507, 509, 516, 515, 525, 513, 514, 513, 511, 514, 515, 517, 520, 516,
        507, 506, 504, 501, 508, 512, 507, 511, 505, 500, 504, 496, 506, 506, 512, 514, 503, 503,
        501, 509, 503, 514, 515, 511, 507, 499, 516, 492,
    ],
    // Queen (936 + positional)
    [
        927, 958, 958, 963, 963, 955, 946, 956, 919, 956, 968, 977, 994, 961, 966, 936, 916, 942,
        945, 985, 983, 971, 955, 945, 939, 958, 960, 981, 993, 976, 993, 972, 918, 964, 955, 983,
        967, 970, 975, 959, 920, 909, 951, 942, 945, 953, 946, 941, 914, 913, 906, 920, 920, 913,
        900, 904, 903, 908, 914, 893, 931, 904, 916, 895,
    ],
    // King (0 + positional)
    [
        -74, -35, -18, -18, -11, 15, 4, -17, -12, 17, 14, 17, 17, 38, 23, 11, 10, 17, 23, 15, 20,
        45, 44, 13, -8, 22, 24, 27, 26, 33, 26, 3, -18, -4, 21, 24, 27, 23, 9, -11, -19, -3, 11,
        21, 23, 16, 7, -9, -27, -11, 4, 13, 14, 4, -5, -17, -53, -34, -21, -11, -28, -14, -24, -43,
    ],
];

const PHASE_INCREMENTS: [i32; PIECE_COUNT] = [0, 1, 1, 2, 4, 0];

pub struct Evaluator {
    gamephase: i32,
    opening_score: i32,
    endgame_score: i32,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            gamephase: 0,
            opening_score: 0,
            endgame_score: 0,
        }
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        self.reset();

        let active_color = board.active_color();

        self.eval_piece_type(active_color, Piece::Pawn, board);
        self.eval_piece_type(active_color, Piece::Knight, board);
        self.eval_piece_type(active_color, Piece::Bishop, board);
        self.eval_piece_type(active_color, Piece::Rook, board);
        self.eval_piece_type(active_color, Piece::Queen, board);
        self.eval_piece_type(active_color, Piece::King, board);

        let opening_phase = self.gamephase.min(24);
        let endgame_phase = 24 - opening_phase;

        (self.opening_score * opening_phase + self.endgame_score * endgame_phase) / 24
    }

    fn eval_piece_type(&mut self, color: Color, piece: Piece, board: &Board) {
        let piece_idx = piece.index();
        let phase_inc = PHASE_INCREMENTS[piece_idx];

        // Active player
        let player_bb = board.bb(color, piece);
        let mut player_opening = 0;
        let mut player_endgame = 0;
        let mut player_count = 0;

        let bitboard_iter = BitboardIterator::new(player_bb);
        for bit in bitboard_iter {
            let square = if color == Color::White { bit ^ 56 } else { bit };

            player_opening += OPENING_TABLES[piece_idx][square as usize];
            player_endgame += ENDGAME_TABLES[piece_idx][square as usize];
            player_count += phase_inc;
        }

        // Opponent player
        let opp_color = !color;
        let opp_bb = board.bb(opp_color, piece);
        let mut opp_opening = 0;
        let mut opp_endgame = 0;
        let mut opp_count = 0;

        let bitboard_iter = BitboardIterator::new(opp_bb);
        for bit in bitboard_iter {
            let square = if opp_color == Color::White {
                bit ^ 56
            } else {
                bit
            };

            opp_opening += OPENING_TABLES[piece_idx][square as usize];
            opp_endgame += ENDGAME_TABLES[piece_idx][square as usize];
            opp_count += phase_inc;
        }

        self.opening_score += player_opening - opp_opening;
        self.endgame_score += player_endgame - opp_endgame;
        self.gamephase += player_count + opp_count;
    }

    fn reset(&mut self) {
        self.opening_score = 0;
        self.endgame_score = 0;
        self.gamephase = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_eval() {
        let mut evaluator = Evaluator::new();
        let board = Board::new("rnbqkb1r/p1pp1ppp/1p3n2/4N3/4P3/8/PPPP1PPP/RNBQKB1R w KQkq - 0 4");

        let start = Instant::now();

        evaluator.evaluate(&board);

        let duration = start.elapsed();
        println!("Test took: {:?}", duration);
    }
}
