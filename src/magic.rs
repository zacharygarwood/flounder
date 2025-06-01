use rand::prelude::ThreadRng;
use rand::RngCore;

use crate::bitboard::{Bitboard, BitboardOperations, SQUARES};
use crate::square::square_to_rank_file;
use crate::square::Square;
use crate::pieces::Piece;

static ROOK_RELEVANT_BITS: [usize; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    12, 11, 11, 11, 11, 11, 11, 12
];

static BISHOP_RELEVANT_BITS: [usize; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6,
    5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
    6, 5, 5, 5, 5, 5, 5, 6
];

static ROOK_MAGICS: [u64; 64] = [
    0x208000211080c000,
    0x8040024610002000,
    0x80100020008148,
    0x1800c0800801000,
    0x20020160010a408,
    0x100340008120100,
    0x80460011000c80,
    0xd08000d08001a500,
    0x1805002080010040,
    0x440005002a000,
    0x8a40808010002000,
    0x9021000910022100,
    0x209001058000501,
    0x20008a4100200,
    0x2004124880200,
    0x3050801100004080,
    0x608000804000,
    0x10104000600140,
    0x4013010010200040,
    0xc808008001004,
    0x1000910004080100,
    0x48880800c000200,
    0x22400c002210081d,
    0x21020a0000440489,
    0x80018080224002,
    0x2088400500208903,
    0x4c08a0200102040,
    0x4000100080800800,
    0x180080800400,
    0xc041000300281400,
    0x2a020400102809,
    0x20801080004100,
    0x240040008080002b,
    0xa902400188802002,
    0x1110408202002350,
    0xa001000a1001900,
    0x908040080800802,
    0x1103104088010420,
    0x1000081184001022,
    0x88000e2800700,
    0x4002a049818008,
    0x2082830200220042,
    0x22002011820042,
    0x202200418a0010,
    0x210501080011002c,
    0x1600120004008080,
    0x4000810268040010,
    0x43000281410022,
    0x2446181020200,
    0xc88400020008e80,
    0xe0004010080040,
    0x81ae201001020900,
    0x2004004480080080,
    0x10c800200240080,
    0x1080180510020c00,
    0xa0098504014200,
    0x100800024403101,
    0x540008023001045,
    0x801a00a00408012,
    0x10c0890010021,
    0x202000811201c0a,
    0x201000884002251,
    0x301000184020003,
    0x3008805400238102,
];

static BISHOP_MAGICS: [u64; 64] = [
    0x9a0a00f19002080,
    0x1010030b00a000,
    0x209434040a480200,
    0x1e2411c204320100,
    0x44142080034004,
    0x20282a0080202,
    0x12081202110000,
    0x2202808082840,
    0x404064ec042140,
    0x1084038c0048,
    0x9102400484600,
    0x880082080201000,
    0x1030820211020000,
    0x2008484130105006,
    0x40412280c220880,
    0x10642022084,
    0x90600028c2d00200,
    0x244092811082200,
    0x32100180208a200,
    0x1000868806004015,
    0x85000090401204,
    0x1006000048040400,
    0x904000100880402,
    0x460c0900420260,
    0x208044008201800,
    0x4010028008020400,
    0x1880010005010,
    0xc004000a0210c4,
    0x1a084004880a018,
    0x1019010452008180,
    0x28022000421205,
    0x4002002aa4888808,
    0x80420a0a0080200,
    0x518033004148400,
    0xc210401002a0800,
    0x20004c0400280210,
    0x9011010400420020,
    0x8085010082008c,
    0x8c0c2412414100,
    0x8084104a02824121,
    0x402401204a011085,
    0xa04040404001200,
    0x4200086888081000,
    0xe22030141088800,
    0x2008206009021880,
    0x80ca0aa101010200,
    0x812004010a014140,
    0x44010242000500,
    0x402a180404440042,
    0x8000848805508020,
    0x80a201c2080060,
    0x5402822880204,
    0x200c100a120014,
    0x1404406448008004,
    0x820891001144008,
    0x5126080600820000,
    0x60a20810843084,
    0x84010108424200,
    0x100101080942,
    0x4000000000940400,
    0x8020680124208200,
    0x4000488900100,
    0x21342020015200a0,
    0x1194108202040410,
];

pub struct Magic {
    pub rook_attack_masks: [Bitboard; 64],
    pub bishop_attack_masks: [Bitboard; 64],
    pub rook_attacks: Vec<Vec<Bitboard>>,
    pub bishop_attacks: Vec<Vec<Bitboard>>,
    pub rook_magics: [u64; 64],
    pub bishop_magics: [u64; 64],
}

impl Magic {
    pub fn new() -> Self {
        // To initialize new magics to Self::init_magics(Piece::Type)
        // Going to use precomputed magics in BISHOP_MAGICS and ROOK_MAGICS
        let bishop_magics = BISHOP_MAGICS;
        let rook_magics = ROOK_MAGICS;
        let (bishop_attack_masks, bishop_attacks) = Self::init_slider_attacks(Piece::Bishop, bishop_magics);
        let (rook_attack_masks, rook_attacks) = Self::init_slider_attacks(Piece::Rook, rook_magics);

        Self {
            rook_attack_masks,
            bishop_attack_masks,
            rook_attacks,
            bishop_attacks,
            rook_magics,
            bishop_magics,
        }
    }

    pub fn get_bishop_attacks(&self, square: Square, mut occupancy: Bitboard) -> Bitboard {
        occupancy &= self.bishop_attack_masks[square as usize];
        occupancy = occupancy.wrapping_mul(self.bishop_magics[square as usize]);
        occupancy >>= 64 - BISHOP_RELEVANT_BITS[square as usize];
        
        self.bishop_attacks[square as usize][occupancy as usize]
    }

    pub fn get_rook_attacks(&self, square: Square, mut occupancy: Bitboard) -> Bitboard {
        occupancy &= self.rook_attack_masks[square as usize];
        occupancy = occupancy.wrapping_mul(self.rook_magics[square as usize]);
        occupancy >>= 64 - ROOK_RELEVANT_BITS[square as usize];

        self.rook_attacks[square as usize][occupancy as usize]
    }

    fn init_magics(piece: Piece) -> [u64; 64] {
        let mut magics = [0; 64];
        for square in 0..SQUARES {
            let relevant_bits = match piece {
                Piece::Bishop => BISHOP_RELEVANT_BITS[square as usize],
                _ => ROOK_RELEVANT_BITS[square as usize],
            };
            magics[square as usize] = Self::find_magic(square, piece, relevant_bits);
        }
        magics
    }

    fn find_magic(square: Square, piece: Piece, relevant_bits: usize) -> u64 {
        let mut occupancies = [Bitboard::empty(); 4096];
        let occupancy_variations = 1 << relevant_bits;

        let mut attacks = [Bitboard::empty(); 4096];
        let attack_mask = Self::generate_attack_mask(piece, square, Bitboard::empty(), false);

        // Generate attack masks for each occupancy variation
        for i in 0..occupancy_variations {
            occupancies[i] = Self::generate_occupancy_board(i, attack_mask);
            attacks[i] = Self::generate_attack_mask(piece, square, occupancies[i], true);
        }

        // Testing magic numbers
        for _random_count in 0..1000000 {
            let magic: u64 = Self::gen_random_number();

            // Reset used attacks
            let mut used_attacks = [Bitboard::empty(); 4096];

            let mut fail = false;
            for i in 0..occupancy_variations {
                let magic_index = occupancies[i].wrapping_mul(magic) >> (64 - relevant_bits);

                if used_attacks[magic_index as usize] == 0 { // Free index in used attacks
                    used_attacks[magic_index as usize] = attacks[i]
                } else if used_attacks[magic_index as usize] != attacks[i] { // Fail on collision
                    fail = true;
                }
            }

            // Found a magic number
            if !fail {
                println!("Magic: 0x{},", format!("{:x}", magic));
                return magic;
            }
        } 
        println!("Failed in magic number creation");
        return 0

    }

    // Returns the attacks masks 
    fn init_slider_attacks(piece: Piece, magics: [u64; 64]) -> ([Bitboard; 64], Vec<Vec<Bitboard>>) {
        // Initialize variables to return
        let mut piece_masks = [Bitboard::empty(); 64];
        let mut piece_attacks = match piece {
            Piece::Bishop => (0..64).map(|_| vec![0; 512]).collect::<Vec<Vec<Bitboard>>>(),
            _ => (0..64).map(|_| vec![0; 4096]).collect::<Vec<Vec<Bitboard>>>(), // Rook is the only other option
        };

        for square in 0..SQUARES {
            // Generate attack mask and store result
            let attack_mask = Self::generate_attack_mask(piece, square, Bitboard::empty(), false);
            piece_masks[square as usize] = attack_mask;
            
            let relevant_bits = match piece {
                Piece::Bishop => BISHOP_RELEVANT_BITS[square as usize],
                _ => ROOK_RELEVANT_BITS[square as usize],
            };

            // Create the piece attacks by mapping a certain (occupancy * magic) shifted to the attacks 
            let occupancy_variations = 1 << relevant_bits;
            for i in 0..occupancy_variations {
                let occupancy = Self::generate_occupancy_board(i, attack_mask);
                let magic_index = occupancy.wrapping_mul(magics[square as usize]) >> (64 - relevant_bits);
                piece_attacks[square as usize][magic_index as usize] = Self::generate_attack_mask(piece, square, occupancy, true);
            }
        }
        (piece_masks, piece_attacks)
    }

    fn generate_occupancy_board(index: usize, attack_mask: Bitboard) -> Bitboard {
        let mut blocker_board: Bitboard = attack_mask;
    
        let mut bit_index: i8 = 0;
        for square in 0..SQUARES {
            if attack_mask & Bitboard::square_to_bitboard(square) != 0 {
                if index & Bitboard::square_to_bitboard(bit_index as u8) as usize == 0 {
                    blocker_board &= !Bitboard::square_to_bitboard(square);
                }
                bit_index += 1;
            }
        }
        blocker_board
    }

    fn generate_attack_mask(piece: Piece, square: u8, blockers: Bitboard, block: bool) -> Bitboard {
        match piece {
            Piece::Bishop => Self::generate_bishop_attack_mask(square, blockers, block),
            _ => Self::generate_rook_attack_mask(square, blockers, block),
        }
    }

    fn generate_rook_attack_mask(square: u8, blockers: Bitboard, block: bool) -> Bitboard {
        let mut mask: Bitboard = 0;
        let (rank, file) = square_to_rank_file(square);
    
        // Generate mask in the bottom direction
        let mut f = file as i8;
        let mut r = rank as i8 - 1;
        while r >= 0 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            r -= 1;
        }
    
        // Generate mask in the left direction
        f = file as i8 - 1;
        r = rank as i8;
        while f >= 0 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            f -= 1;
        }
    
        // Generate mask in the up direction
        f = file as i8;
        r = rank as i8 + 1;
        while f >= 0 && r < 8 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            r += 1;
        }
    
        // Generate mask in the right direction
        f = file as i8 + 1;
        r = rank as i8;
        while f < 8 && r < 8 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            f += 1;
        }

        if !block {
            mask &= !Bitboard::rank_file_to_edge_mask(rank, file);
        }
        mask
    }
    
    fn generate_bishop_attack_mask(square: Square, blockers: Bitboard, block: bool) -> Bitboard {
        let mut mask: Bitboard = 0;
        let (rank, file) = square_to_rank_file(square);
    
        // Generate mask in the bottom-left direction
        let mut f = file as i8 - 1;
        let mut r = rank as i8 - 1;
        while f >= 0 && r >= 0 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            f -= 1;
            r -= 1;
        }
    
        // Generate mask in the bottom-right direction
        f = file as i8 + 1;
        r = rank as i8 - 1;
        while f < 8 && r >= 0 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            f += 1;
            r -= 1;
        }
    
        // Generate mask in the top-left direction
        f = file as i8 - 1;
        r = rank as i8 + 1;
        while f >= 0 && r < 8 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            f -= 1;
            r += 1;
        }
    
        // Generate mask in the top-right direction
        f = file as i8 + 1;
        r = rank as i8 + 1;
        while f < 8 && r < 8 {
            let square_bb = Bitboard::rank_file_to_bitboard(r as u8, f as u8);
            mask |= square_bb;
            if block && blockers & square_bb != 0 {
                break;
            }
            f += 1;
            r += 1;
        }
        
        if !block {
            mask &= !Bitboard::rank_file_to_edge_mask(rank, file);
        }
        mask
    }

    fn gen_random_number() -> u64 {
        let n1: u64 = Self::gen_u64();
        let n2: u64 = Self::gen_u64();
        let n3: u64 = Self::gen_u64();
        n1 & n2 & n3
    }


    fn gen_u64() -> u64 {
        let mut random = ThreadRng::default();
        let u1: u64 = random.next_u64() & 0xFFFF;
        let u2: u64 = random.next_u64() & 0xFFFF;
        let u3: u64 = random.next_u64() & 0xFFFF;
        let u4: u64 = random.next_u64() & 0xFFFF;
        u1 | (u2 << 16) | (u3 << 32) | (u4 << 48)
    }
}
