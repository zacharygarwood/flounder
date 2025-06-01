use crate::board::Board;
use crate::bitboard::{Bitboard, BitboardIterator, BitboardOperations, RANK_2, RANK_3, RANK_6, RANK_7, WHITE_KING_SIDE, WHITE_QUEEN_SIDE, BLACK_KING_SIDE, BLACK_QUEEN_SIDE};
use crate::lookup::LookupTable;
use crate::pieces::{Piece, Color, PromotionPieceIterator};
use crate::moves::{Move, MoveType, NORTH, EAST, SOUTH, WEST};
use crate::square::{Square, C1, C8, E1, E8, G1, G8};

pub struct MoveGenerator {
    pub lookup: LookupTable
}

impl MoveGenerator {
    pub fn new() -> Self {
        Self {
            lookup: LookupTable::init(),
        }
    }

    pub fn generate_moves(&self, board: &Board) -> Vec<Move> {
        let mut moves = Vec::new();
        
        // Generate moves for each piece type
        self.generate_pseudo_legal_castles(board, &mut moves);
        self.generate_pseudo_legal_pawn_moves(board, &mut moves);
        self.generate_pseudo_legal_moves(board, Piece::King, &mut moves);
        self.generate_pseudo_legal_moves(board, Piece::Knight, &mut moves);
        self.generate_pseudo_legal_moves(board, Piece::Bishop, &mut moves);
        self.generate_pseudo_legal_moves(board, Piece::Rook, &mut moves);
        self.generate_pseudo_legal_moves(board, Piece::Queen, &mut moves);

        let king_square = self.king_square(board);
        let pinned_pieces = self.get_pinned_pieces(board, king_square);
        let checkers = self.attacks_to(board, king_square);

        moves.retain(|mv| self.is_legal(board, mv, checkers, pinned_pieces, king_square));
    
        moves
    }

    pub fn generate_quiescence_moves(&self, board: &Board) -> Vec<Move> {
        let mut moves = self.generate_moves(board);
        
        moves.retain(|mv| (self.is_capture(mv) || self.is_promotion(mv) || self.is_check(board, mv)));

        moves
    }
    
    fn generate_pseudo_legal_pawn_moves(&self, board: &Board, moves: &mut Vec<Move>) {
        use crate::pieces::Piece::*;
    
        let color = board.active_color();
        let pawns = board.bb(color, Pawn);
        let direction = PawnDirection::new(color);
    
        self.generate_quiet_pawn_pushes(board, pawns, direction, moves);
        self.generate_pawn_captures(board, pawns, direction, moves);
        self.generate_en_passants(board, pawns, direction, moves);
        self.generate_promotions(board, pawns, direction, moves);
    }
    
    fn generate_quiet_pawn_pushes(&self, board: &Board, pawns: Bitboard, direction: PawnDirection, moves: &mut Vec<Move>) {
        let pawns = pawns & !direction.rank_7;
        let empty_squares = board.bb_empty();
    
        // Generate single pawn pushes
        let single_pushes = pawns.shift(direction.north) & empty_squares;
    
        // Generate double pawn pushes
        let double_pawns = single_pushes & direction.rank_3;
        let double_pushes = double_pawns.shift(direction.north) & empty_squares;
    
        // Store moves
        self.extract_pawn_moves(single_pushes, direction.north, MoveType::Quiet, moves);
        self.extract_pawn_moves(double_pushes, direction.north + direction.north, MoveType::Quiet, moves);
    }
    
    fn generate_pawn_captures(&self, board: &Board, pawns: Bitboard, direction: PawnDirection, moves: &mut Vec<Move>) {
        let pawns = pawns & !direction.rank_7;
        let color = board.active_color();
    
        // Generate valid pawn attacks
        let enemy_pieces = board.bb_color(!color);
        let left_pawn_attacks = pawns.shift(direction.north + WEST) & enemy_pieces;
        let right_pawn_attacks = pawns.shift(direction.north + EAST) & enemy_pieces;
        
        // Store moves
        self.extract_pawn_moves(left_pawn_attacks, direction.north + WEST, MoveType::Capture, moves);
        self.extract_pawn_moves(right_pawn_attacks, direction.north + EAST, MoveType::Capture, moves);
    }

    fn generate_en_passants(&self, board: &Board, pawns: Bitboard, direction: PawnDirection, moves: &mut Vec<Move>) {
        // Bitboard with en passant target set, or empty
        let en_passant_target = match board.en_passant_target {
            Some(square) => Bitboard::square_to_bitboard(square),
            None => Bitboard::empty(),
        };

        // Generate valid pawn en passant attacks
        let left_pawn_attacks = pawns.shift(direction.north + WEST) & en_passant_target;
        let right_pawn_attacks = pawns.shift(direction.north + EAST) & en_passant_target;

        // Store moves
        self.extract_pawn_moves(left_pawn_attacks, direction.north + WEST, MoveType::EnPassant, moves);
        self.extract_pawn_moves(right_pawn_attacks, direction.north + EAST, MoveType::EnPassant, moves);

    }

    fn generate_promotions(&self, board: &Board, pawns: Bitboard, direction: PawnDirection, moves: &mut Vec<Move>) {
        // Only look at pawns that can promote
        let pawns = pawns & direction.rank_7;
        let color = board.active_color();
        let enemy_pieces = board.bb_color(!color);
        let empty_squares = board.bb_empty();
    
        // Generate single pawn pushes
        let single_pushes = pawns.shift(direction.north) & empty_squares;

        // Generate valid pawn attacks
        let left_pawn_attacks = pawns.shift(direction.north + WEST) & enemy_pieces;
        let right_pawn_attacks = pawns.shift(direction.north + EAST) & enemy_pieces;
        
        // Store moves
        self.extract_promotions(single_pushes, direction.north, MoveType::Promotion, moves);
        self.extract_promotions(left_pawn_attacks, direction.north + WEST, MoveType::Promotion, moves);
        self.extract_promotions(right_pawn_attacks, direction.north + EAST, MoveType::Promotion, moves);
    }
    
    fn extract_pawn_moves(&self, bitboard: Bitboard, offset: i8, move_type: MoveType, moves: &mut Vec<Move>) {
        let iter = BitboardIterator::new(bitboard);
        for square in iter {
            let mv = Move::new((square as i8 - offset) as u8, square, Piece::Pawn, move_type);
            moves.push(mv);
        }
    }

    fn extract_promotions(&self, bitboard: Bitboard, offset: i8, move_type: MoveType, moves: &mut Vec<Move>) {
        let bb_iter = BitboardIterator::new(bitboard);
        let promotion_pieces = PromotionPieceIterator::new();
        for square in bb_iter {
            for piece in promotion_pieces {
                let mv = Move::new((square as i8 - offset) as u8, square, piece, move_type);
                moves.push(mv);
            }
        }
    }

    fn generate_pseudo_legal_castles(&self, board: &Board, moves: &mut Vec<Move>) {
        let color = board.active_color();
        let all_pieces = board.bb_all();
        let (king_side_rights, queen_side_rights) = board.castling_ability(color);

        let (king_side_mask, queen_side_mask) = match color {
            Color::White => (WHITE_KING_SIDE, WHITE_QUEEN_SIDE),
            Color::Black => (BLACK_KING_SIDE, BLACK_QUEEN_SIDE),
        };

        let king_side_occupancy = king_side_mask & all_pieces;
        let queen_side_occupancy = queen_side_mask & all_pieces;

        // Castle king side if they have the rights and nothing blocks
        if king_side_rights && king_side_occupancy == 0 {
            self.extract_castles(color, Piece::King, MoveType::Castle, moves);
        }

        // Castle queen side if they have the rights and nothing blocks
        if queen_side_rights && queen_side_occupancy == 0 {
            self.extract_castles(color, Piece::Queen, MoveType::Castle, moves);
        }
    }

    fn extract_castles(&self, color: Color, side_to_castle: Piece, move_type: MoveType, moves: &mut Vec<Move>) {
        let (starting_square, king_side_square, queen_side_square) = match color {
            Color::White => (E1, G1, C1),
            Color::Black => (E8, G8, C8),
        };

        match side_to_castle {
            Piece::King => {
                let mv = Move::new(starting_square, king_side_square as u8, Piece::King, move_type);
                moves.push(mv);
            },
            Piece::Queen => {
                let mv = Move::new(starting_square, queen_side_square as u8, Piece::King, move_type);
                moves.push(mv);
            },
            _ => {} // Only care about King and Queen for king side and queen side castling respectively
        };
    }
    
    fn generate_pseudo_legal_moves(&self, board: &Board, piece: Piece, moves: &mut Vec<Move>) {
        let color = board.active_color();
        let pieces = board.bb(color, piece);
        let enemy_pieces = board.bb_color(!color);
        let empty_squares = board.bb_empty();
    
        let iter = BitboardIterator::new(pieces);
        for square in iter {
            let destinations = match piece {
                Piece:: Knight | Piece::King => self.lookup.non_sliding_moves(square, piece),
                _ => self.lookup.sliding_moves(square, board.bb_all(), piece)
            };

            let quiet_moves = destinations & empty_squares;
            let capture_moves = destinations & enemy_pieces;

            self.extract_moves(quiet_moves, square, piece, MoveType::Quiet, moves);
            self.extract_moves(capture_moves, square, piece, MoveType::Capture, moves);
        }
    }
    
    fn extract_moves(&self, bitboard: Bitboard, from: u8, piece_type:Piece, move_type: MoveType, moves: &mut Vec<Move>) {
        let iter = BitboardIterator::new(bitboard);
        for square in iter {
            let mv = Move::new(from, square, piece_type, move_type);
            moves.push(mv);
        }
    }

    // Returns a bitboard with all pieces attacking a certain square
    pub fn attacks_to(&self, board: &Board, square: Square) -> Bitboard {
        let color = board.active_color();
        let square_bb = Bitboard::square_to_bitboard(square);
        let occupancy = board.bb_all() & !board.bb(color, Piece::King);

        // Get all attacks from square
        let pawn_attacks = match color {
            Color::White => square_bb.shift(NORTH + WEST) | square_bb.shift(NORTH + EAST),
            Color::Black => square_bb.shift(SOUTH + WEST) | square_bb.shift(SOUTH + EAST),
        };

        let knight_attacks = self.lookup.non_sliding_moves(square, Piece::Knight);
        let bishop_attacks = self.lookup.sliding_moves(square, occupancy, Piece::Bishop);
        let rook_attacks = self.lookup.sliding_moves(square, occupancy, Piece::Rook);
        let queen_attacks = self.lookup.sliding_moves(square, occupancy, Piece::Queen);
        let king_attacks = self.lookup.non_sliding_moves(square, Piece::King);

        // Get relevant pieces that can attack the square
        let pawns = pawn_attacks & board.bb(!color, Piece::Pawn);
        let knights = knight_attacks & board.bb(!color, Piece::Knight);
        let bishops = bishop_attacks & board.bb(!color, Piece::Bishop);
        let rooks = rook_attacks & board.bb(!color, Piece::Rook);
        let king = king_attacks & board.bb(!color, Piece::King);
        let queens = queen_attacks & board.bb(!color, Piece::Queen);

        pawns | knights | bishops | rooks | king | queens
    }

    fn get_pinned_pieces(&self, board: &Board, king_square: Square) -> Bitboard {
        let color = board.active_color();
        let occupancy = board.bb_all();
        let king_bb = board.bb(color, Piece::King);

        let enemy_bishops = board.bb(!color, Piece::Bishop);
        let enemy_rooks = board.bb(!color, Piece::Rook);
        let enemy_queens = board.bb(!color, Piece::Queen);

        let bishop_attackers = self.lookup.sliding_moves(king_square, enemy_bishops, Piece::Bishop) & enemy_bishops;
        let rook_attackers = self.lookup.sliding_moves(king_square, enemy_rooks, Piece::Rook) & enemy_rooks;
        let queen_attackers = self.lookup.sliding_moves(king_square, enemy_queens, Piece::Queen) & enemy_queens;

        let pinners = bishop_attackers | rook_attackers | queen_attackers;
        let mut pinned_pieces = Bitboard::empty();

        let iter = BitboardIterator::new(pinners);
        for pinner in iter {
            // We don't want the pinner or king to be considered a pinned piece
            let ignore = Bitboard::square_to_bitboard(pinner) | king_bb;
            let potential_pinned_pieces = self.lookup.between(pinner, king_square, true) & occupancy & !ignore;

            if potential_pinned_pieces.count_ones() == 1 {
                pinned_pieces |= potential_pinned_pieces;
            }
        }
        pinned_pieces

    }

    pub fn king_square(&self, board: &Board) -> Square {
        let color = board.active_color();
        board.bb(color, Piece::King).trailing_zeros() as Square
    }

    fn is_legal(&self, board: &Board, mv: &Move, checkers: Bitboard, pinned_pieces: Bitboard, king_square: Square) -> bool {
        let is_castle = mv.move_type == MoveType::Castle;
        let is_king = mv.piece_type == Piece::King;
        
        if is_king && !is_castle {
            self.is_legal_king_move(board, mv)
        } else {
            self.is_legal_non_king_move(board, mv, checkers, pinned_pieces, king_square)
        }
    }

    fn is_legal_king_move(&self, board: &Board, mv: &Move) -> bool {
        self.attacks_to(board, mv.to) == 0
    }

    fn is_legal_non_king_move(&self, board: &Board, mv: &Move, checkers: Bitboard, pinned_pieces: Bitboard, king_square: Square) -> bool {
        let num_checks = checkers.count_ones();

        // When there are two or more checks the only legal moves are king moves
        if num_checks > 1 {
            return false;
        }

        // Special moves that have their own validation
        if mv.move_type == MoveType::EnPassant {
            return self.is_legal_en_passant(board, mv, king_square);
        }

        if mv.move_type == MoveType::Castle {
            return self.is_legal_castle(board, mv, num_checks);
        }

        let pinned = self.is_pinned(mv, pinned_pieces);

        // If there is one check then the move can either capture the attacking piece or block the check
        if num_checks == 1 {
            let attacker = checkers.trailing_zeros() as u8;
            let to_bb = Bitboard::square_to_bitboard(mv.to);

            // Capture attacking piece
            if mv.to == attacker {
                return !pinned;
            } else { // Move piece to block check
                let attacking_ray = self.lookup.between(attacker, king_square, true);
                let is_piece_on_ray = attacking_ray & to_bb != 0;

                return !pinned && is_piece_on_ray;
            }
        }

        // If not pinned the piece is free to move since the king is not in check
        if pinned {
            return self.is_legal_pinned_move(mv, king_square);
        } else {
            return true;
        }
    }

    fn is_pinned(&self, mv: &Move, pinned_pieces: Bitboard) -> bool {
        let from_bb = Bitboard::square_to_bitboard(mv.from);
        
        (pinned_pieces & from_bb) != 0
    }

    fn is_legal_pinned_move(&self, mv: &Move, king_square: Square) -> bool {
        let king_bb = Bitboard::square_to_bitboard(king_square);
        let ray = self.lookup.between(mv.to, mv.from, false);
        let is_king_on_ray = (ray & king_bb) != 0;
        
        is_king_on_ray
    }

    fn is_legal_en_passant(&self, board: &Board, mv: &Move, king_square: Square) -> bool {
        let mut board = *board;
        let color = board.active_color();
        
        let en_passant_square = match color {
            Color::White => mv.to - 8,
            Color::Black => mv.to + 8,
        };

        let temp_move = Move::new(mv.from, mv.to, Piece::Pawn, MoveType::Capture);

        // Remove pawn being captured to perform needed legality checks
        board.remove_piece(!color, Piece::Pawn, en_passant_square);

        let pinned_pieces = self.get_pinned_pieces(&board, king_square);
        let checkers = self.attacks_to(&board, king_square);
        let is_legal = self.is_legal_non_king_move(&board, &temp_move, checkers, pinned_pieces, king_square);

        // Add pawn that was captured back
        board.add_piece(!color, Piece::Pawn, en_passant_square);

        is_legal
    }

    fn is_legal_castle(&self, board: &Board, mv: &Move, num_checks: u32) -> bool {
        if num_checks != 0 {
            return false;
        }

        let color = board.active_color();
        let (king_side_square, king_side_checks, queen_side_checks) = match color {
            Color::White => (G1, vec![5, 6], vec![2,3]),
            Color::Black => (G8, vec![61, 62], vec![58, 59]),
        };

        let is_king_side = mv.to == king_side_square;

        let squares_to_check = match is_king_side {
            true => king_side_checks,
            false => queen_side_checks,
        };
        
        for square in squares_to_check {
            if self.attacks_to(board, square) != 0 {
                return false;
            }
        }

        true
    }

    fn is_capture(&self, mv: &Move) -> bool {
        mv.move_type == MoveType::Capture || mv.move_type == MoveType::EnPassant
    }

    fn is_promotion(&self, mv: &Move) -> bool {
        mv.move_type == MoveType::Promotion
    }

    fn is_check(&self, board: &Board, mv: &Move) -> bool {
        let new_board = board.clone_with_move(mv);
        self.attacks_to(&new_board, self.king_square(&new_board)) != 0
    }

    pub fn run_perft(&self, board: &Board, depth: usize) -> usize {
        self.perft(board, depth)
    }

    fn perft(&self, board: &Board, depth: usize) -> usize {
        let mut nodes = 0;
        let moves = self.generate_moves(board);

        if depth == 0 {
            return 1;
        }

        if depth == 1 {
            return moves.len();
        }

        for mv in moves {
            let new_board = board.clone_with_move(&mv);
            nodes += self.perft(&new_board, depth - 1);
        }

        nodes
    }

    pub fn divide(&self, board: &Board, depth: usize){

        let moves = self.generate_moves(board);
        let mut total = 0;

        println!("Moves: {}", moves.len());

        for mv in moves {
            let new_board = board.clone_with_move(&mv);
            let result = self.run_perft(&new_board, depth-1);
            mv.print();
            print!(": {}\n", result);
            total += result;
        }
        println!("Total: {}", total);
    }
}

#[derive(Copy, Clone)]
struct PawnDirection {
    rank_7: Bitboard,
    rank_3: Bitboard,
    north: i8,
}

impl PawnDirection {
    fn new(color: Color) -> Self {
        let rank_7 = match color {
            Color::White => RANK_7,
            Color::Black => RANK_2,
        };
        let rank_3 = match color {
            Color::White => RANK_3,
            Color::Black => RANK_6,
        };
        let north = match color {
            Color::White => NORTH,
            Color::Black => SOUTH,
        };
        Self { rank_7, rank_3, north }
    }
}


#[cfg(test)]
mod tests {
    use crate::board::Board;
    use crate::move_gen::MoveGenerator;

    // Positions and results can be found here
    // https://www.chessprogramming.org/Perft_Results
    #[test]
    fn perft_position_1() {
        let board = Board::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let move_gen = MoveGenerator::new();

        assert_eq!(move_gen.run_perft(&board, 0), 1);
        assert_eq!(move_gen.run_perft(&board, 1), 20);
        assert_eq!(move_gen.run_perft(&board, 2), 400);
        assert_eq!(move_gen.run_perft(&board, 3), 8902);
        assert_eq!(move_gen.run_perft(&board, 4), 197281);
        assert_eq!(move_gen.run_perft(&board, 5), 4865609);
        assert_eq!(move_gen.run_perft(&board, 6), 119060324);
    }

    #[test]
    fn perft_position_2() {
        let board = Board::new("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        let move_gen = MoveGenerator::new();

        assert_eq!(move_gen.run_perft(&board, 1), 48);
        assert_eq!(move_gen.run_perft(&board, 2), 2039);
        assert_eq!(move_gen.run_perft(&board, 3), 97862);
        assert_eq!(move_gen.run_perft(&board, 4), 4085603);
        assert_eq!(move_gen.run_perft(&board, 5), 193690690);
    }

    #[test]
    fn perft_position_3() {
        let board = Board::new("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
        let move_gen = MoveGenerator::new();

        assert_eq!(move_gen.run_perft(&board, 1), 14);
        assert_eq!(move_gen.run_perft(&board, 2), 191);
        assert_eq!(move_gen.run_perft(&board, 3), 2812);
        assert_eq!(move_gen.run_perft(&board, 4), 43238);
        assert_eq!(move_gen.run_perft(&board, 5), 674624);
        assert_eq!(move_gen.run_perft(&board, 6), 11030083);
        assert_eq!(move_gen.run_perft(&board, 7), 178633661);
    }

    #[test]
    fn perft_position_4() {
        let board = Board::new("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        let move_gen = MoveGenerator::new();

        assert_eq!(move_gen.run_perft(&board, 1), 6);
        assert_eq!(move_gen.run_perft(&board, 2), 264);
        assert_eq!(move_gen.run_perft(&board, 3), 9467);
        assert_eq!(move_gen.run_perft(&board, 4), 422333);
        assert_eq!(move_gen.run_perft(&board, 5), 15833292);
        assert_eq!(move_gen.run_perft(&board, 6), 706045033);
    }

    #[test]
    fn perft_position_5() {
        let board = Board::new("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
        let move_gen = MoveGenerator::new();

        assert_eq!(move_gen.run_perft(&board, 1), 44);
        assert_eq!(move_gen.run_perft(&board, 2), 1486);
        assert_eq!(move_gen.run_perft(&board, 3), 62379);
        assert_eq!(move_gen.run_perft(&board, 4), 2103487);
        assert_eq!(move_gen.run_perft(&board, 5), 89941194);
    }

    #[test]
    fn perft_position_6() {
        let board = Board::new("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10");
        let move_gen = MoveGenerator::new();

        assert_eq!(move_gen.run_perft(&board, 0), 1);
        assert_eq!(move_gen.run_perft(&board, 1), 46);
        assert_eq!(move_gen.run_perft(&board, 2), 2079);
        assert_eq!(move_gen.run_perft(&board, 3), 89890);
        assert_eq!(move_gen.run_perft(&board, 4), 3894594);
        assert_eq!(move_gen.run_perft(&board, 5), 164075551);
    }
}
