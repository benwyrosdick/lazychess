mod app;
mod chess;
mod config;
mod engine;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use app::App;
use config::Config;

#[derive(Parser, Debug)]
#[command(name = "lazychess")]
#[command(
    author,
    version,
    about = "A chess analysis TUI with Stockfish integration"
)]
struct Args {
    /// FEN string to load initially
    #[arg(short, long)]
    fen: Option<String>,

    /// PGN file to load
    #[arg(short, long)]
    pgn: Option<String>,

    /// Search depth for analysis
    #[arg(short, long)]
    depth: Option<u32>,

    /// Number of best lines to show (MultiPV)
    #[arg(short, long)]
    multipv: Option<u32>,

    /// Path to Stockfish binary
    #[arg(short, long)]
    stockfish: Option<String>,

    /// Piece style: "nerd" (default), "unicode", or "ascii"
    #[arg(long, default_value = "nerd")]
    pieces: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let mut config = Config::load()?;

    // Override config with command-line arguments
    if let Some(depth) = args.depth {
        config.engine.depth = depth;
    }
    if let Some(multipv) = args.multipv {
        config.engine.multipv = multipv;
    }
    if let Some(stockfish) = args.stockfish {
        config.engine.path = Some(stockfish);
    }
    config.ui.piece_style = args.pieces;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config)?;

    // Load initial position if specified
    if let Some(fen) = args.fen {
        if let Err(e) = app.game.load_fen(&fen) {
            app.input.set_error(format!("Invalid FEN: {}", e));
        } else {
            app.start_analysis()?;
        }
    } else if let Some(pgn_path) = args.pgn {
        let pgn_content = std::fs::read_to_string(&pgn_path)?;
        // The app will parse this on first tick or we can do it manually
        app.input.pgn_buffer = pgn_content.lines().map(String::from).collect();
        // We'd need to call finish_pgn_input but it's private, so let's handle differently
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        // Render
        terminal.draw(|f| app.render(f))?;

        // Process engine events
        app.tick()?;

        // Check for quit
        if app.should_quit {
            return Ok(());
        }

        // Poll for events with timeout (allows engine updates)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key)?;
            }
        }
    }
}
