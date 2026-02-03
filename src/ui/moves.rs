use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::chess::Game;

/// Move history widget
pub struct MovesWidget<'a> {
    game: &'a Game,
    /// Scroll offset for the move list
    _scroll_offset: usize,
}

impl<'a> MovesWidget<'a> {
    pub fn new(game: &'a Game, scroll_offset: usize) -> Self {
        Self {
            game,
            _scroll_offset: scroll_offset,
        }
    }
}

impl Widget for MovesWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title(" Moves ");

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let formatted_moves = self.game.formatted_moves();
        let current_idx = self.game.current_index();

        let mut lines: Vec<Line> = Vec::new();

        // Track which move is highlighted
        let mut move_counter = 0;

        for (move_num, white_move, black_move) in &formatted_moves {
            let mut spans: Vec<Span> = Vec::new();

            // Move number
            spans.push(Span::styled(
                format!("{:>3}. ", move_num),
                Style::default().fg(Color::DarkGray),
            ));

            // White's move
            move_counter += 1;
            let white_style = if move_counter == current_idx {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(format!("{:<7}", white_move), white_style));

            // Black's move (if any)
            if let Some(black) = black_move {
                move_counter += 1;
                let black_style = if move_counter == current_idx {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(format!("{:<7}", black), black_style));
            }

            lines.push(Line::from(spans));
        }

        // If no moves yet, show a placeholder
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No moves yet",
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Calculate scroll
        let visible_height = inner.height as usize;
        let total_lines = lines.len();

        // Auto-scroll to keep current position visible
        let current_line = formatted_moves
            .iter()
            .take_while(|(num, _, _)| {
                let white_idx = (*num - 1) * 2 + 1;
                white_idx < current_idx
            })
            .count();

        let scroll = if current_line >= visible_height {
            current_line.saturating_sub(visible_height / 2)
        } else {
            0
        }
        .min(total_lines.saturating_sub(visible_height));

        let paragraph = Paragraph::new(lines).scroll((scroll as u16, 0));
        paragraph.render(inner, buf);
    }
}
