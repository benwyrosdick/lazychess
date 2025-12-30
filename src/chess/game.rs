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
            pos = pos.play(m).expect("Stored move should be valid");

            if white_move.is_none() {
                // This is white's move (or black's first move if initial position was black to move)
                if self.initial_position.turn() == Color::White {
                    white_move = Some(san);
                } else {
                    // Black to move initially
                    result.push((move_num, "...".to_string(), Some(san)));
                    move_num += 1;
                }
            } else {
                // This is black's move
                result.push((move_num, white_move.take().unwrap(), Some(san)));
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
}

/// Convert a square to algebraic notation
pub fn square_to_string(sq: Square) -> String {
    sq.to_string()
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
