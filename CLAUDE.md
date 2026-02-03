# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build              # Build the project
cargo run                # Run the TUI application
cargo install --path .   # Install locally
```

## Prerequisites

- Rust toolchain
- Stockfish chess engine (auto-detected from PATH, or specify with `--stockfish` flag)

## Architecture Overview

This is a Rust TUI application for chess analysis using Stockfish. Built with ratatui for the terminal UI.

### Core Modules

- **`src/app.rs`** - Main application state (`App` struct). Handles keyboard input, manages popups, coordinates between game state and engine. Contains the main event loop logic and UI rendering orchestration.

- **`src/chess/game.rs`** - Chess game state wrapper around the `shakmaty` library. Manages position, move history, and navigation (go back/forward through moves). Supports FEN loading and basic PGN parsing.

- **`src/engine/uci.rs`** - UCI protocol implementation for communicating with Stockfish. Spawns the engine as a subprocess, sends commands via stdin, and reads analysis info from stdout using a background thread with mpsc channels.

- **`src/config.rs`** - Configuration management with TOML serialization. Config stored at `~/.config/lazychess/config.toml`. Supports engine settings (depth, MultiPV, threads, hash) and UI settings (flip board, piece style).

- **`src/ui/`** - Widget implementations for ratatui:
  - `board.rs` - Chess board rendering with piece styles (unicode, nerd font, ascii)
  - `analysis.rs` - Engine analysis lines display with eval scores
  - `moves.rs` - Move history panel
  - `input.rs` - Command/move input handling with different modes (Normal, Command, FEN, PGN)
  - `help.rs` - Help popup and status bar

### Key Patterns

- Engine communication is async via mpsc channels - `Engine::try_recv()` is polled each tick
- Game navigation maintains full move history and rebuilds position when navigating
- Input has multiple modes: Normal (shortcuts), Command (move entry), FEN input, PGN input
- Analysis lines are stored by MultiPV index and updated incrementally from engine info messages
