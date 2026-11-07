//! consertion — a fast, keyboard- and mouse-friendly converter for the terminal.

mod app;
mod catalog;
mod cli;
mod clipboard;
mod components;
mod convert;
mod files;
mod num;
mod rates;
mod screens;
mod settings;
mod theme;
mod translate;

use ratatui::crossterm::event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture};
use ratatui::crossterm::execute;
use std::io::stdout;

pub fn set_mouse(on: bool) {
    // Errors are ignored: some consoles lack these modes, and the app still works by keyboard.
    let _ = if on { execute!(stdout(), EnableMouseCapture) } else { execute!(stdout(), DisableMouseCapture) };
}

fn restore_extras() {
    set_mouse(false);
    let _ = execute!(stdout(), DisableBracketedPaste);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = cli::run(&args) {
        std::process::exit(code);
    }
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        eprintln!("consertion: the app needs a terminal — for scripts use e.g. consertion 5 mi km (see --help)");
        std::process::exit(2);
    }
    let mut app = app::App::new();
    let mut terminal = ratatui::init();
    // ratatui's hook restores raw mode/alt screen; also undo our extra terminal modes on panic.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_extras();
        hook(info);
    }));
    let _ = execute!(stdout(), EnableBracketedPaste);
    set_mouse(app.settings.mouse);
    let result = app.run(&mut terminal);
    restore_extras();
    ratatui::restore();
    result
}
