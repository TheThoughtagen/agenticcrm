mod app;
mod event;
mod ui;
mod views;
mod widgets;

use std::io;
use std::panic;

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    event::{self as ct_event, Event, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use app::App;
use event::handle_key;
use ui::draw;

pub fn run() -> anyhow::Result<()> {
    // Install panic hook that restores terminal before printing panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    let mut terminal = init_terminal()?;
    let mut app = App::new()?;

    while app.running {
        terminal.draw(|frame| draw(frame, &mut app))?;

        if poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = ct_event::read()? {
                if let Some(msg) = handle_key(&app, key) {
                    app.update(msg);
                }
            }
        }
    }

    restore_terminal()?;
    Ok(())
}

fn init_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
