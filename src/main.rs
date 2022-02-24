mod defer;
mod line;
mod log;
mod state;
mod ui;

use {
    anyhow::{anyhow, Result},
    crossterm::{
        cursor,
        event::{self, Event},
        execute, terminal,
    },
    defer::defer,
    log::log,
    state::State,
    std::{env, io, panic},
};

fn main() -> Result<()> {
    // Normal panic reporting gets mangled when we're in raw mode, so write to log instead
    panic::set_hook(Box::new(|panic_info| {
        match (
            panic_info.location(),
            panic_info.payload().downcast_ref::<&str>(),
        ) {
            (Some(location), Some(msg)) => log!("PANIC {} {}", location, msg),
            (Some(location), None) => log!("PANIC {} ?", location),
            (None, Some(msg)) => log!("PANIC ? {}", msg),
            (None, None) => log!("PANIC ? ?"),
        };
    }));
    terminal::enable_raw_mode()?;
    defer! { terminal::disable_raw_mode().unwrap(); }
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    defer! { execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap(); }
    let mut s = State::new(env::args().nth(1).ok_or(anyhow!("File required"))?)?;
    let mut out = io::stdout();
    execute!(out, cursor::SetCursorShape(cursor::CursorShape::Line))?;
    let mut size = terminal::size()?;
    ui::draw(&mut out, &s, size)?;
    loop {
        match event::read()? {
            Event::Key(event) => {
                if !s.handle(event) {
                    break;
                }
            }
            Event::Mouse(_) => continue,
            Event::Resize(x, y) => size = (x, y),
        }
        ui::draw(&mut out, &s, size)?;
    }
    Ok(())
}
