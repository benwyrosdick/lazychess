#![allow(dead_code)]

use anyhow::{Context, Result};
use shakmaty::{fen::Fen, san::San, CastlingMode, Chess, Color, Move, Piece, Position, Role, Square};

/// Represents the full game state with move history
#[derive(Debug, Clone)]
pub struct Game {
    /// Initial position (for reset)
    initial_position: Chess,
    /// Current position
    position: Chess,
    /// List of moves played (from initial position)
    moves: Vec<Move>,
    /// Current position index (for navigation). Points to the position AFTER moves[index-1]
    /// 0 = initial position, moves.len() = current/latest position
    current_index: usize,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// Create a new game from the starting position
    pub fn new() -> Self {
        Self {
            initial_position: Chess::default(),
            position: Chess::default(),
            moves: Vec::new(),
            current_index: 0,
        }
    }

    /// Create a game from a FEN string
    pub fn from_fen(fen: &str) -> Result<Self> {
        let fen: Fen = fen.parse().context("Invalid FEN string")?;
        let position: Chess = fen
            .into_position(CastlingMode::Standard)
            .context("Invalid position")?;

        Ok(Self {
            initial_position: position.clone(),
            position,
            moves: Vec::new(),
            current_index: 0,
        })
    }

    /// Get the current FEN string
    pub fn to_fen(&self) -> String {
        Fen::from_position(self.position.clone(), shakmaty::EnPassantMode::Legal).to_string()
    }

    /// Get the current position
    pub fn position(&self) -> &Chess {
        &self.position
    }

    /// Get whose turn it is
    pub fn turn(&self) -> Color {
        self.position.turn()
    }

    /// Get the piece at a square
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.position.board().piece_at(square)
    }

    /// Get all legal moves in the current position
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::new();
        self.position.legal_moves().into_iter().for_each(|m| moves.push(m));
        moves
    }

    /// Try to make a move from SAN notation (e.g., "e4", "Nf3", "O-O")
    pub fn make_move_san(&mut self, san_str: &str) -> Result<Move> {
        let san: San = san_str.parse().context("Invalid move notation")?;
        let m = san
            .to_move(&self.position)
            .context("Illegal move for current position")?;

        self.make_move(m.clone())?;
        Ok(m)
    }

    /// Make a move
    pub fn make_move(&mut self, m: Move) -> Result<()> {
        // If we're not at the end, truncate the move list (overwrite mode)
        if self.current_index < self.moves.len() {
            self.moves.truncate(self.current_index);
        }

        // Apply the move
        let new_position = self.position.clone().play(&m).context("Illegal move")?;
        self.position = new_position;
        self.moves.push(m);
        self.current_index = self.moves.len();

        Ok(())
    }

    /// Get the move history
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Get the current position index
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Check if we're at the start of the game
    pub fn is_at_start(&self) -> bool {
        self.current_index == 0
    }

    /// Check if we're at the latest position
    pub fn is_at_end(&self) -> bool {
        self.current_index == self.moves.len()
    }

    /// Go to the previous position
    pub fn go_back(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.rebuild_position();
            true
        } else {
            false
        }
    }

    /// Go to the next position
    pub fn go_forward(&mut self) -> bool {
        if self.current_index < self.moves.len() {
            self.current_index += 1;
            self.rebuild_position();
            true
        } else {
            false
        }
    }

    /// Go to the start
    pub fn go_to_start(&mut self) {
        self.current_index = 0;
        self.rebuild_position();
    }

    /// Go to the end (latest position)
    pub fn go_to_end(&mut self) {
        self.current_index = self.moves.len();
        self.rebuild_position();
    }

    /// Rebuild the position from moves up to current_index
    fn rebuild_position(&mut self) {
        self.position = self.initial_position.clone();
        for m in &self.moves[..self.current_index] {
            self.position = self.position.clone().play(m).expect("Stored move should be valid");
        }
    }

    /// Get the last move (if any) - the move that led to the current position
    pub fn last_move(&self) -> Option<&Move> {
        if self.current_index > 0 {
            self.moves.get(self.current_index - 1)
        } else {
            None
        }
    }

    /// Reset to a new game
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Load from FEN, resetting the game
    pub fn load_fen(&mut self, fen: &str) -> Result<()> {
        *self = Self::from_fen(fen)?;
        Ok(())
    }

    /// Get move as SAN string
    pub fn move_to_san(&self, m: &Move, position: &Chess) -> String {
        let san = San::from_move(position, m);
        san.to_string()
    }

    /// Get the formatted move list for display
    pub fn formatted_moves(&self) -> Vec<(usize, String, Option<String>)> {
        let mut result = Vec::new();
        let mut pos = self.initial_position.clone();
        let mut move_num = 1;
        let mut white_move: Option<String> = None;

        for m in &self.moves {
            let san = San::from_move(&pos, m).to_string();
            let is_white_move = pos.turn() == Color::White;
            pos = pos.play(m).expect("Stored move should be valid");

            if is_white_move {
                // White's move - store it and wait for black's response
                white_move = Some(san);
            } else {
                // Black's move
                if let Some(w) = white_move.take() {
                    // Pair with white's move
                    result.push((move_num, w, Some(san)));
                } else {
                    // Black moved first (from a FEN position)
                    result.push((move_num, "...".to_string(), Some(san)));
                }
                move_num += 1;
            }
        }

        // Push any remaining white move
        if let Some(w) = white_move {
            result.push((move_num, w, None));
        }

        result
    }

    /// Check if game is over
    pub fn is_game_over(&self) -> bool {
        self.position.is_game_over()
    }

    /// Get the game outcome if over
    pub fn outcome(&self) -> Option<shakmaty::Outcome> {
        self.position.outcome()
    }

    /// Get captured pieces for each side
    /// Returns (white_captured, black_captured) where each is a list of roles
    /// white_captured = pieces that white has captured (black pieces that are gone)
    /// black_captured = pieces that black has captured (white pieces that are gone)
    pub fn captured_pieces(&self) -> (Vec<Role>, Vec<Role>) {
        // Standard starting material for each side
        let starting_material: [(Role, u8); 5] = [
            (Role::Queen, 1),
            (Role::Rook, 2),
            (Role::Bishop, 2),
            (Role::Knight, 2),
            (Role::Pawn, 8),
        ];

        let board = self.position.board();

        // Count current pieces for each side
        let mut white_captured: Vec<Role> = Vec::new(); // Black pieces captured by white
        let mut black_captured: Vec<Role> = Vec::new(); // White pieces captured by black

        for (role, starting_count) in starting_material {
            // Count how many of this piece type each side currently has
            let white_count = board.by_color(Color::White).intersect(board.by_role(role)).count() as u8;
            let black_count = board.by_color(Color::Black).intersect(board.by_role(role)).count() as u8;

            // White captured = starting black pieces - current black pieces
            let white_captures = starting_count.saturating_sub(black_count);
            for _ in 0..white_captures {
                white_captured.push(role);
            }

            // Black captured = starting white pieces - current white pieces
            let black_captures = starting_count.saturating_sub(white_count);
            for _ in 0..black_captures {
                black_captured.push(role);
            }
        }

        (white_captured, black_captured)
    }

    /// Get material advantage in centipawns (positive = white advantage)
    pub fn material_balance(&self) -> i32 {
        let piece_values: [(Role, i32); 5] = [
            (Role::Queen, 900),
            (Role::Rook, 500),
            (Role::Bishop, 330),
            (Role::Knight, 320),
            (Role::Pawn, 100),
        ];

        let board = self.position.board();
        let mut balance = 0;

        for (role, value) in piece_values {
            let white_count = board.by_color(Color::White).intersect(board.by_role(role)).count() as i32;
            let black_count = board.by_color(Color::Black).intersect(board.by_role(role)).count() as i32;
            balance += (white_count - black_count) * value;
        }

        balance
    }
}

/// Convert a square to algebraic notation
pub fn square_to_string(sq: Square) -> String {
    sq.to_string()
}

/// Piece style for rendering
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PieceStyle {
    /// Standard Unicode chess symbols (♔♕♖♗♘♙)
    #[default]
    Unicode,
    /// Nerd Font icons (requires a Nerd Font)
    NerdFont,
    /// ASCII letters (K, Q, R, B, N, P)
    Ascii,
}

/// Get the character for a piece based on style
pub fn piece_to_char(piece: Piece, style: PieceStyle) -> char {
    match style {
        PieceStyle::Unicode => piece_to_unicode(piece),
        PieceStyle::NerdFont => piece_to_nerd_font(piece),
        PieceStyle::Ascii => piece_to_ascii(piece),
    }
}

/// Get the Unicode character for a piece
pub fn piece_to_unicode(piece: Piece) -> char {
    match (piece.color, piece.role) {
        (Color::White, Role::King) => '♔',
        (Color::White, Role::Queen) => '♕',
        (Color::White, Role::Rook) => '♖',
        (Color::White, Role::Bishop) => '♗',
        (Color::White, Role::Knight) => '♘',
        (Color::White, Role::Pawn) => '♙',
        (Color::Black, Role::King) => '♚',
        (Color::Black, Role::Queen) => '♛',
        (Color::Black, Role::Rook) => '♜',
        (Color::Black, Role::Bishop) => '♝',
        (Color::Black, Role::Knight) => '♞',
        (Color::Black, Role::Pawn) => '♟',
    }
}

/// Get the Nerd Font character for a piece (Font Awesome chess icons)
/// Note: These are the same icon for both colors - we differentiate by color styling
pub fn piece_to_nerd_font(piece: Piece) -> char {
    match piece.role {
        Role::King => '\u{f0857}',   // nf-fa-chess_king
        Role::Queen => '\u{f085a}',  // nf-fa-chess_queen
        Role::Rook => '\u{f085b}',   // nf-fa-chess_rook
        Role::Bishop => '\u{f085c}', // nf-fa-chess_bishop
        Role::Knight => '\u{f0858}', // nf-fa-chess_knight
        Role::Pawn => '\u{f0859}',   // nf-fa-chess_pawn
    }
}

/// Get the ASCII character for a piece
pub fn piece_to_ascii(piece: Piece) -> char {
    let c = match piece.role {
        Role::King => 'K',
        Role::Queen => 'Q',
        Role::Rook => 'R',
        Role::Bishop => 'B',
        Role::Knight => 'N',
        Role::Pawn => 'P',
    };
    if piece.color == Color::White {
        c
    } else {
        c.to_ascii_lowercase()
    }
}
