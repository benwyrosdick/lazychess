use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};
use shakmaty::{File, Move, Piece, Rank, Role, Square};

use crate::chess::{piece_to_char, Game, PieceStyle};
use crate::config::UiConfig;

/// Chess board widget
pub struct BoardWidget<'a> {
    game: &'a Game,
    config: &'a UiConfig,
    last_move: Option<&'a Move>,
    piece_style: PieceStyle,
    /// Show captured pieces inside the board pane
    show_captured: bool,
}

impl<'a> BoardWidget<'a> {
    pub fn new(game: &'a Game, config: &'a UiConfig) -> Self {
        Self {
            game,
            config,
            last_move: game.last_move(),
            piece_style: config.get_piece_style(),
            show_captured: true,
        }
    }

    fn render_captured_pieces(&self, area: Rect, buf: &mut Buffer, show_white_captures: bool) {
        if area.width < 2 || area.height < 1 {
            return;
        }

        let (white_captured, black_captured) = self.game.captured_pieces();
        let material_balance = self.game.material_balance();

        // Get the pieces to display
        let pieces = if show_white_captures {
            &white_captured
        } else {
            &black_captured
        };

        // Calculate material advantage for this side
        let advantage = if show_white_captures {
            if material_balance > 0 {
                Some(material_balance / 100)
            } else {
                None
            }
        } else {
            if material_balance < 0 {
                Some(-material_balance / 100)
            } else {
                None
            }
        };

        // Build horizontal display string
        let piece_order = [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn];
        let mut display_parts: Vec<(String, Color)> = Vec::new();

        for role in piece_order {
            let count = pieces.iter().filter(|&&r| r == role).count();
            if count > 0 {
                let color = if show_white_captures {
                    shakmaty::Color::Black
                } else {
                    shakmaty::Color::White
                };
                let piece = Piece { color, role };
                let piece_char = piece_to_char(piece, self.piece_style);

                let fg_color = if piece.color == shakmaty::Color::White {
                    Color::White
                } else {
                    Color::DarkGray
                };

                // Show piece repeated by count
                for _ in 0..count {
                    display_parts.push((format!("{}", piece_char), fg_color));
                }
            }
        }

        // Render horizontally
        let mut x = area.x;
        for (text, fg_color) in &display_parts {
            if x >= area.x + area.width {
                break;
            }
            buf.set_string(x, area.y, text, Style::default().fg(*fg_color));
            x += text.chars().count() as u16;
        }

        // Show material advantage at the end if this side is ahead
        if let Some(adv) = advantage {
            if adv > 0 && x < area.x + area.width {
                buf.set_string(
                    x,
                    area.y,
                    format!("+{}", adv),
                    Style::default().fg(Color::Green),
                );
            }
        }
    }

    fn get_square_color(&self, file: File, rank: Rank) -> Color {
        let is_light = (file as u8 + rank as u8) % 2 == 1;
        if is_light {
            Color::Rgb(240, 217, 181) // Light square
        } else {
            Color::Rgb(181, 136, 99) // Dark square
        }
    }

    fn get_highlight_color(&self, file: File, rank: Rank) -> Color {
        let is_light = (file as u8 + rank as u8) % 2 == 1;
        if is_light {
            Color::Rgb(205, 210, 106) // Light highlight
        } else {
            Color::Rgb(170, 162, 58) // Dark highlight
        }
    }

    fn is_highlighted(&self, square: Square) -> bool {
        if !self.config.highlight_last_move {
            return false;
        }

        if let Some(m) = self.last_move {
            let from = m.from().unwrap_or(m.to());
            let to = m.to();
            return square == from || square == to;
        }
        false
    }
}

impl Widget for BoardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Board ");

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 26 || inner.height < 10 {
            // Not enough space to render board
            return;
        }

        // Calculate cell size (4 chars wide, 2 chars tall per square for better visibility)
        let cell_width = 4u16;
        let cell_height = 2u16;

        // Board dimensions
        let board_width = 8 * cell_width;
        let board_height = 8 * cell_height;

        // Coordinates width
        let coord_width = if self.config.show_coordinates { 3 } else { 0 };
        let _coord_height = if self.config.show_coordinates { 1 } else { 0 };

        // Reserve space for captured pieces (1 line each at top and bottom)
        let captured_height = if self.show_captured { 1 } else { 0 };

        // Center the board horizontally in the available space
        let total_width = board_width + coord_width;
        let start_x = inner.x + (inner.width.saturating_sub(total_width)) / 2 + coord_width;
        let start_y = inner.y + captured_height;

        // Render top captured pieces (opponent's pieces when board is not flipped)
        if self.show_captured {
            let top_captures_white = self.config.flip_board;
            let top_area = Rect::new(start_x, inner.y, board_width, 1);
            self.render_captured_pieces(top_area, buf, top_captures_white);
        }

        // Determine rank/file order based on flip
        let ranks: Vec<Rank> = if self.config.flip_board {
            (0..8).map(|r| Rank::new(r)).collect()
        } else {
            (0..8).rev().map(|r| Rank::new(r)).collect()
        };

        let files: Vec<File> = if self.config.flip_board {
            (0..8).rev().map(|f| File::new(f)).collect()
        } else {
            (0..8).map(|f| File::new(f)).collect()
        };

        // Render the board
        for (row_idx, &rank) in ranks.iter().enumerate() {
            for row_line in 0..cell_height {
                let y = start_y + (row_idx as u16 * cell_height) + row_line;

                if y >= inner.y + inner.height {
                    continue;
                }

                // Render rank coordinate (only on first line of cell, centered vertically)
                if self.config.show_coordinates && row_line == cell_height / 2 {
                    let rank_char = (b'1' + rank as u8) as char;
                    buf.set_string(
                        start_x.saturating_sub(2),
                        y,
                        format!("{}", rank_char),
                        Style::default().fg(Color::DarkGray),
                    );
                }

                for (col_idx, &file) in files.iter().enumerate() {
                    let x = start_x + (col_idx as u16 * cell_width);
                    let square = Square::from_coords(file, rank);

                    // Determine background color
                    let bg_color = if self.is_highlighted(square) {
                        self.get_highlight_color(file, rank)
                    } else {
                        self.get_square_color(file, rank)
                    };

                    let style = Style::default().bg(bg_color);

                    // Fill the entire cell with background color first
                    let blank_cell = "    ";
                    buf.set_string(x, y, blank_cell, style);

                    // Get piece at square (only show on middle row)
                    if row_line == cell_height / 2 {
                        if let Some(piece) = self.game.piece_at(square) {
                            let piece_char = piece_to_char(piece, self.piece_style);

                            // Determine piece color
                            let fg_color = if piece.color == shakmaty::Color::White {
                                Color::White
                            } else {
                                Color::Black
                            };

                            let piece_style = Style::default().fg(fg_color).bg(bg_color);

                            // Render piece centered in cell (position 1 of 0-3)
                            buf.set_string(x + 1, y, format!("{}", piece_char), piece_style);
                        }
                    }
                }
            }
        }

        // Render file coordinates
        if self.config.show_coordinates {
            let y = start_y + board_height;
            if y < inner.y + inner.height {
                for (col_idx, &file) in files.iter().enumerate() {
                    let x = start_x + (col_idx as u16 * cell_width) + 1;
                    let file_char = (b'a' + file as u8) as char;
                    buf.set_string(
                        x,
                        y,
                        format!("{}", file_char),
                        Style::default().fg(Color::DarkGray),
                    );
                }
            }
        }

        // Render bottom captured pieces (our pieces when board is not flipped)
        if self.show_captured {
            let bottom_captures_white = !self.config.flip_board;
            let coord_offset = if self.config.show_coordinates { 1 } else { 0 };
            let bottom_y = start_y + board_height + coord_offset;
            if bottom_y < inner.y + inner.height {
                let bottom_area = Rect::new(start_x, bottom_y, board_width, 1);
                self.render_captured_pieces(bottom_area, buf, bottom_captures_white);
            }
        }
    }
}

/// Widget showing whose turn and game status
pub struct StatusWidget<'a> {
    game: &'a Game,
}

impl<'a> StatusWidget<'a> {
    pub fn new(game: &'a Game) -> Self {
        Self { game }
    }
}

impl Widget for StatusWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let turn = self.game.turn();
        let turn_str = if turn == shakmaty::Color::White {
            "White"
        } else {
            "Black"
        };

        let status = if self.game.is_game_over() {
            match self.game.outcome() {
                Some(shakmaty::Outcome::Decisive { winner }) => {
                    if winner == shakmaty::Color::White {
                        "White wins!".to_string()
                    } else {
                        "Black wins!".to_string()
                    }
                }
                Some(shakmaty::Outcome::Draw) => "Draw".to_string(),
                None => format!("{} to move", turn_str),
            }
        } else {
            format!("{} to move", turn_str)
        };

        // Use nerd font chess king for turn indicator
        let turn_indicator = if turn == shakmaty::Color::White {
            "\u{f43f}" // nf-fa-chess_king
        } else {
            "\u{f43f}" // same icon, different context
        };

        let indicator_color = if turn == shakmaty::Color::White {
            Color::White
        } else {
            Color::DarkGray
        };

        buf.set_string(
            area.x + 1,
            area.y,
            turn_indicator,
            Style::default().fg(indicator_color),
        );
        buf.set_string(
            area.x + 3,
            area.y,
            &status,
            Style::default().fg(Color::White),
        );
    }
}


