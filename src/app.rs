use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::chess::Game;
use crate::config::Config;
use crate::engine::{Engine, EngineEvent};
use crate::ui::{
    AnalysisState, AnalysisWidget, BoardWidget, DepthPopup, HelpBarWidget, HelpPopup,
    ImportPopup, InputMode, InputState, InputWidget, MovesWidget, MultiPVPopup, StatusWidget,
};

/// Popup state
#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    None,
    Help,
    Import,
    Depth(String),
    MultiPV(String),
}

/// Main application state
pub struct App {
    /// Chess game state
    pub game: Game,
    /// Configuration
    pub config: Config,
    /// Stockfish engine (optional - may not be available)
    pub engine: Option<Engine>,
    /// Analysis state
    pub analysis: AnalysisState,
    /// Input state
    pub input: InputState,
    /// Current popup
    pub popup: Popup,
    /// Should quit?
    pub should_quit: bool,
    /// Move scroll offset
    pub move_scroll: usize,
    /// Last position sent to engine (to detect changes)
    last_fen: String,
}

impl App {
    /// Create a new application
    pub fn new(config: Config) -> Result<Self> {
        let game = Game::new();
        let analysis = AnalysisState::new(config.engine.depth);

        // Try to start the engine
        let engine = match config.stockfish_path() {
            Some(path) => match Engine::new(&path) {
                Ok(mut e) => {
                    // Configure the engine
                    let _ = e.set_option("MultiPV", &config.engine.multipv.to_string());
                    let _ = e.set_option("Threads", &config.engine.threads.to_string());
                    let _ = e.set_option("Hash", &config.engine.hash.to_string());
                    let _ = e.set_option("Contempt", &config.engine.contempt.to_string());
                    Some(e)
                }
                Err(e) => {
                    eprintln!("Warning: Failed to start Stockfish: {}", e);
                    None
                }
            },
            None => {
                eprintln!("Warning: Stockfish not found in PATH");
                None
            }
        };

        let last_fen = game.to_fen();

        let mut app = Self {
            game,
            config,
            engine,
            analysis,
            input: InputState::default(),
            popup: Popup::None,
            should_quit: false,
            move_scroll: 0,
            last_fen,
        };

        // Start initial analysis
        app.start_analysis()?;

        Ok(app)
    }

    /// Start or restart analysis for the current position
    pub fn start_analysis(&mut self) -> Result<()> {
        if let Some(ref mut engine) = self.engine {
            // Stop any current analysis
            engine.stop()?;

            // Clear previous analysis
            self.analysis.clear();
            self.analysis.is_running = true;
            self.analysis.is_paused = false;

            // Set up the position
            let fen = self.game.to_fen();
            engine.set_position(Some(&fen), &[])?;
            self.last_fen = fen;

            // Start analysis
            engine.go_depth(self.config.engine.depth)?;
        }
        Ok(())
    }

    /// Stop analysis
    pub fn stop_analysis(&mut self) -> Result<()> {
        if let Some(ref mut engine) = self.engine {
            engine.stop()?;
            self.analysis.is_running = false;
        }
        Ok(())
    }

    /// Toggle analysis pause
    pub fn toggle_pause(&mut self) -> Result<()> {
        if self.analysis.is_paused {
            // Resume
            self.start_analysis()?;
        } else {
            // Pause
            self.stop_analysis()?;
            self.analysis.is_paused = true;
        }
        Ok(())
    }

    /// Process engine events
    pub fn process_engine_events(&mut self) {
        if let Some(ref mut engine) = self.engine {
            while let Some(event) = engine.try_recv() {
                match event {
                    EngineEvent::Info(info) => {
                        self.analysis.update(info);
                    }
                    EngineEvent::BestMove(_) => {
                        self.analysis.is_running = false;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Handle a keyboard event
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle Ctrl+C globally
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        // Handle popup-specific input
        match &self.popup {
            Popup::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                        self.popup = Popup::None;
                    }
                    _ => {}
                }
                return Ok(());
            }
            Popup::Import => {
                match key.code {
                    KeyCode::Esc => {
                        self.popup = Popup::None;
                    }
                    KeyCode::Char('f') => {
                        self.popup = Popup::None;
                        self.input.enter_fen_mode();
                    }
                    KeyCode::Char('p') => {
                        self.popup = Popup::None;
                        self.input.enter_pgn_mode();
                    }
                    KeyCode::Char('n') => {
                        self.popup = Popup::None;
                        self.game.reset();
                        self.start_analysis()?;
                        self.input.set_message("New game started");
                    }
                    _ => {}
                }
                return Ok(());
            }
            Popup::Depth(input) => {
                let mut input = input.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.popup = Popup::None;
                    }
                    KeyCode::Enter => {
                        if let Ok(depth) = input.parse::<u32>() {
                            if depth > 0 && depth <= 100 {
                                self.config.engine.depth = depth;
                                self.analysis.target_depth = depth;
                                self.popup = Popup::None;
                                self.start_analysis()?;
                                self.input.set_message(format!("Depth set to {}", depth));
                            } else {
                                self.input.set_error("Depth must be between 1 and 100");
                            }
                        } else {
                            self.input.set_error("Invalid depth value");
                        }
                        self.popup = Popup::None;
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        input.push(c);
                        self.popup = Popup::Depth(input);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        self.popup = Popup::Depth(input);
                    }
                    _ => {}
                }
                return Ok(());
            }
            Popup::MultiPV(input) => {
                let mut input = input.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.popup = Popup::None;
                    }
                    KeyCode::Enter => {
                        if let Ok(multipv) = input.parse::<u32>() {
                            if multipv > 0 && multipv <= 10 {
                                self.config.engine.multipv = multipv;
                                if let Some(ref mut engine) = self.engine {
                                    let _ = engine.set_option("MultiPV", &multipv.to_string());
                                }
                                self.popup = Popup::None;
                                self.start_analysis()?;
                                self.input.set_message(format!("MultiPV set to {}", multipv));
                            } else {
                                self.input.set_error("MultiPV must be between 1 and 10");
                            }
                        } else {
                            self.input.set_error("Invalid MultiPV value");
                        }
                        self.popup = Popup::None;
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        input.push(c);
                        self.popup = Popup::MultiPV(input);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        self.popup = Popup::MultiPV(input);
                    }
                    _ => {}
                }
                return Ok(());
            }
            Popup::None => {}
        }

        // Handle input mode
        if self.input.is_input_mode() {
            match key.code {
                KeyCode::Esc => {
                    self.input.exit_mode();
                }
                KeyCode::Enter => {
                    self.handle_input_submit()?;
                }
                KeyCode::Backspace => {
                    self.input.backspace();
                }
                KeyCode::Delete => {
                    self.input.delete();
                }
                KeyCode::Left => {
                    self.input.move_left();
                }
                KeyCode::Right => {
                    self.input.move_right();
                }
                KeyCode::Home => {
                    self.input.move_start();
                }
                KeyCode::End => {
                    self.input.move_end();
                }
                KeyCode::Char(c) => {
                    self.input.insert(c);
                }
                _ => {}
            }
            return Ok(());
        }

        // Normal mode shortcuts
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.popup = Popup::Help;
            }
            KeyCode::Char('i') => {
                self.popup = Popup::Import;
            }
            KeyCode::Char('f') => {
                self.config.ui.flip_board = !self.config.ui.flip_board;
            }
            KeyCode::Char('p') => {
                self.toggle_pause()?;
            }
            KeyCode::Char('d') => {
                self.popup = Popup::Depth(String::new());
            }
            KeyCode::Char('m') => {
                self.popup = Popup::MultiPV(String::new());
            }
            KeyCode::Char('y') => {
                self.copy_fen_to_clipboard();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.game.go_back() {
                    self.start_analysis()?;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.game.go_forward() {
                    self.start_analysis()?;
                }
            }
            KeyCode::Home => {
                self.game.go_to_start();
                self.start_analysis()?;
            }
            KeyCode::End => {
                self.game.go_to_end();
                self.start_analysis()?;
            }
            KeyCode::Enter | KeyCode::Char(':') => {
                self.input.enter_command_mode();
            }
            KeyCode::Char(c @ '1'..='9') => {
                // Play the best move from the selected analysis line
                let line_idx = (c as usize) - ('1' as usize);
                self.play_analysis_line(line_idx)?;
            }
            KeyCode::Char(c) if c.is_alphabetic() => {
                // Start typing a move
                self.input.enter_command_mode();
                self.input.insert(c);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle input submission
    fn handle_input_submit(&mut self) -> Result<()> {
        match self.input.mode {
            InputMode::Command | InputMode::Normal => {
                let input = self.input.take();
                self.input.exit_mode();

                if input.is_empty() {
                    return Ok(());
                }

                // Check for commands
                if input.starts_with(":fen ") {
                    let fen = input.strip_prefix(":fen ").unwrap().trim();
                    match self.game.load_fen(fen) {
                        Ok(_) => {
                            self.input.set_message("Position loaded from FEN");
                            self.start_analysis()?;
                        }
                        Err(e) => {
                            self.input.set_error(format!("Invalid FEN: {}", e));
                        }
                    }
                } else if input == ":pgn" {
                    self.input.enter_pgn_mode();
                } else {
                    // Try to parse as a move
                    match self.game.make_move_san(&input) {
                        Ok(_) => {
                            self.start_analysis()?;
                        }
                        Err(e) => {
                            self.input.set_error(format!("Invalid move: {}", e));
                        }
                    }
                }
            }
            InputMode::Fen => {
                let input = self.input.take();
                self.input.exit_mode();

                let fen = input.strip_prefix(":fen ").unwrap_or(&input).trim();
                if !fen.is_empty() {
                    match self.game.load_fen(fen) {
                        Ok(_) => {
                            self.input.set_message("Position loaded from FEN");
                            self.start_analysis()?;
                        }
                        Err(e) => {
                            self.input.set_error(format!("Invalid FEN: {}", e));
                        }
                    }
                }
            }
            InputMode::Pgn => {
                let line = self.input.take();

                // Empty line or double-Enter ends PGN input
                if line.is_empty() {
                    self.finish_pgn_input()?;
                } else {
                    self.input.pgn_buffer.push(line);
                }
            }
        }

        Ok(())
    }

    /// Finish PGN input and parse the game
    fn finish_pgn_input(&mut self) -> Result<()> {
        let pgn_text = self.input.pgn_buffer.join("\n");
        self.input.exit_mode();

        if pgn_text.trim().is_empty() {
            return Ok(());
        }

        match self.parse_pgn(&pgn_text) {
            Ok(_) => {
                self.input.set_message("Game loaded from PGN");
                self.start_analysis()?;
            }
            Err(e) => {
                self.input.set_error(format!("Invalid PGN: {}", e));
            }
        }

        Ok(())
    }

    /// Parse PGN and load the game (simple parser)
    fn parse_pgn(&mut self, pgn: &str) -> Result<()> {
        // Simple PGN parser - extract moves from the movetext
        // Skip headers (lines starting with '[')
        // Parse moves like "1. e4 e5 2. Nf3 Nc6"

        self.game.reset();

        let mut in_comment = false;
        let mut in_variation = 0;

        for token in pgn.split_whitespace() {
            // Skip headers
            if token.starts_with('[') {
                continue;
            }

            // Handle comments
            if token.starts_with('{') {
                in_comment = true;
            }
            if token.ends_with('}') {
                in_comment = false;
                continue;
            }
            if in_comment {
                continue;
            }

            // Handle variations
            if token.starts_with('(') {
                in_variation += 1;
            }
            if token.ends_with(')') {
                in_variation -= 1;
                continue;
            }
            if in_variation > 0 {
                continue;
            }

            // Skip move numbers (e.g., "1.", "12.")
            if token.ends_with('.') {
                continue;
            }
            // Skip "..." for black moves
            if token == "..." {
                continue;
            }

            // Skip result markers
            if token == "1-0" || token == "0-1" || token == "1/2-1/2" || token == "*" {
                continue;
            }

            // Skip annotations like "!" "?" "!!" etc.
            let clean_move = token
                .trim_end_matches('!')
                .trim_end_matches('?')
                .trim_end_matches('+')
                .trim_end_matches('#');

            if clean_move.is_empty() {
                continue;
            }

            // Try to make the move
            self.game.make_move_san(clean_move)?;
        }

        Ok(())
    }

    /// Play the first move from an analysis line
    fn play_analysis_line(&mut self, line_idx: usize) -> Result<()> {
        // Check if we have this analysis line
        if let Some(info) = self.analysis.lines.get(line_idx) {
            if let Some(uci_move) = info.pv.first() {
                // Parse the UCI move and convert to a legal move
                if let Ok(uci) = uci_move.parse::<shakmaty::uci::UciMove>() {
                    if let Ok(m) = uci.to_move(self.game.position()) {
                        // Convert to SAN for make_move_san
                        let san = shakmaty::san::San::from_move(self.game.position(), &m);
                        match self.game.make_move_san(&san.to_string()) {
                            Ok(_) => {
                                self.start_analysis()?;
                            }
                            Err(e) => {
                                self.input.set_error(format!("Failed to play move: {}", e));
                            }
                        }
                    } else {
                        self.input.set_error("Invalid move from engine");
                    }
                } else {
                    self.input.set_error("Failed to parse engine move");
                }
            } else {
                self.input.set_error(format!("No moves in line {}", line_idx + 1));
            }
        } else {
            self.input.set_error(format!("No analysis line {}", line_idx + 1));
        }
        Ok(())
    }

    /// Copy current FEN to clipboard
    fn copy_fen_to_clipboard(&mut self) {
        let fen = self.game.to_fen();
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(&fen) {
                Ok(_) => {
                    self.input.set_message("FEN copied to clipboard");
                }
                Err(e) => {
                    self.input.set_error(format!("Failed to copy: {}", e));
                }
            },
            Err(e) => {
                self.input.set_error(format!("Clipboard error: {}", e));
            }
        }
    }

    /// Render the UI
    pub fn render(&self, frame: &mut Frame) {
        let size = frame.area();

        // Main layout: vertical split
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title bar
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Input bar
                Constraint::Length(1), // Help bar
            ])
            .split(size);

        // Title bar
        let title = format!(
            " lazychess {}",
            if self.engine.is_some() {
                ""
            } else {
                "(no engine)"
            }
        );
        let title_widget = ratatui::widgets::Paragraph::new(title)
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
        frame.render_widget(title_widget, main_chunks[0]);

        // Main content: horizontal split into (board + analysis) | moves
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(40),    // Left side (board + analysis)
                Constraint::Length(22), // Move history
            ])
            .split(main_chunks[1]);

        // Left panel: board on top, analysis below
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(20), // Board (8*2 rows + 2 border + 1 coords + 1 status)
                Constraint::Min(8),     // Analysis
            ])
            .split(content_chunks[0]);

        // Board area (board + status)
        let board_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),   // Board
                Constraint::Length(1), // Status
            ])
            .split(left_chunks[0]);

        // Render board
        let board_widget = BoardWidget::new(&self.game, &self.config.ui);
        frame.render_widget(board_widget, board_chunks[0]);

        // Render status
        let status_widget = StatusWidget::new(&self.game);
        frame.render_widget(status_widget, board_chunks[1]);

        // Render analysis panel
        let analysis_widget = AnalysisWidget::new(&self.analysis, self.game.position(), self.config.engine.multipv);
        frame.render_widget(analysis_widget, left_chunks[1]);

        // Render move history
        let moves_widget = MovesWidget::new(&self.game, self.move_scroll);
        frame.render_widget(moves_widget, content_chunks[1]);

        // Render input bar
        let input_widget = InputWidget::new(&self.input);
        frame.render_widget(input_widget, main_chunks[2]);

        // Render help bar
        let help_widget = HelpBarWidget::new(self.input.is_input_mode());
        frame.render_widget(help_widget, main_chunks[3]);

        // Render popup if any
        match &self.popup {
            Popup::Help => {
                let area = HelpPopup::centered_rect(60, 80, size);
                frame.render_widget(HelpPopup::new(), area);
            }
            Popup::Import => {
                let area = HelpPopup::centered_rect(40, 40, size);
                frame.render_widget(ImportPopup::new(), area);
            }
            Popup::Depth(input) => {
                let area = HelpPopup::centered_rect(30, 30, size);
                frame.render_widget(DepthPopup::new(self.config.engine.depth, input), area);
            }
            Popup::MultiPV(input) => {
                let area = HelpPopup::centered_rect(35, 30, size);
                frame.render_widget(MultiPVPopup::new(self.config.engine.multipv, input), area);
            }
            Popup::None => {}
        }
    }

    /// Main application tick - process events
    pub fn tick(&mut self) -> Result<()> {
        self.process_engine_events();
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Save config on exit
        let _ = self.config.save();
    }
}
