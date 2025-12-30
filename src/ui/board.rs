use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};
use shakmaty::{File, Move, Rank, Square};

use crate::chess::{piece_to_unicode, Game};
use crate::config::UiConfig;

/// Chess board widget
pub struct BoardWidget<'a> {
    game: &'a Game,
    config: &'a UiConfig,
    last_move: Option<&'a Move>,
}

impl<'a> BoardWidget<'a> {
    pub fn new(game: &'a Game, config: &'a UiConfig) -> Self {
        Self {
            game,
            config,
            last_move: game.last_move(),
        }
    }

    fn get_square_color(&self, file: File, rank: Rank) -> Color {
        let is_light = (file as u8 + rank as u8) % 2 == 1;
        if is_light {
            Color::Rgb(240, 217, 181) // Light square
        } else {
            Color::Rgb(181, 136, 99)  // Dark square
        }
    }

    fn get_highlight_color(&self, file: File, rank: Rank) -> Color {
        let is_light = (file as u8 + rank as u8) % 2 == 1;
        if is_light {
            Color::Rgb(205, 210, 106) // Light highlight
        } else {
            Color::Rgb(170, 162, 58)  // Dark highlight
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

        if inner.width < 18 || inner.height < 10 {
            // Not enough space to render board
            return;
        }

        // Calculate cell size (2 chars wide, 1 char tall per square)
        let cell_width = 2u16;
        let cell_height = 1u16;

        // Board dimensions
        let _board_width = 8 * cell_width;
        let board_height = 8 * cell_height;

        // Coordinates width
        let coord_width = if self.config.show_coordinates { 2 } else { 0 };
        let _coord_height = if self.config.show_coordinates { 1 } else { 0 };

        // Center the board in the available space
        let start_x = inner.x + coord_width;
        let start_y = inner.y;

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
            let y = start_y + (row_idx as u16 * cell_height);

            // Render rank coordinate
            if self.config.show_coordinates && start_x >= 2 {
                let rank_char = (b'1' + rank as u8) as char;
                buf.set_string(
                    start_x - 2,
                    y,
                    format!("{} ", rank_char),
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

                // Get piece at square
                let piece_char = self
                    .game
                    .piece_at(square)
                    .map(piece_to_unicode)
                    .unwrap_or(' ');

                // Determine piece color (black pieces on dark squares need contrast)
                let fg_color = if let Some(piece) = self.game.piece_at(square) {
                    if piece.color == shakmaty::Color::White {
                        Color::White
                    } else {
                        Color::Black
                    }
                } else {
                    bg_color
                };

                let style = Style::default().fg(fg_color).bg(bg_color);

                // Render the square (2 chars wide: space + piece)
                let cell = format!("{} ", piece_char);
                buf.set_string(x, y, &cell, style);
            }
        }

        // Render file coordinates
        if self.config.show_coordinates {
            let y = start_y + board_height;
            if y < inner.y + inner.height {
                for (col_idx, &file) in files.iter().enumerate() {
                    let x = start_x + (col_idx as u16 * cell_width);
                    let file_char = (b'a' + file as u8) as char;
                    buf.set_string(
                        x,
                        y,
                        format!("{} ", file_char),
                        Style::default().fg(Color::DarkGray),
                    );
                }
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

        let turn_indicator = if turn == shakmaty::Color::White {
            "♔"
        } else {
            "♚"
        };

        let text = format!("{} {}", turn_indicator, status);
        let style = Style::default().fg(Color::White);

        buf.set_string(area.x, area.y, &text, style);
    }
}
