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
    state::State,
    std::{env, io},
};

fn main() -> Result<()> {
    let mut s = State::new(env::args().nth(1).ok_or(anyhow!("File required"))?)?;
    let mut out = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(out, terminal::EnterAlternateScreen)?;
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
    execute!(out, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
