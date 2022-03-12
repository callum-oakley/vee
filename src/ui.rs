use {
    crate::state::{Point, State},
    anyhow::{bail, Result},
    crossterm::{
        cursor, queue,
        style::{self, Color},
        terminal::{self, ClearType},
    },
    std::{io, iter},
    unicode_width::UnicodeWidthChar,
};

fn draw_text<W>(mut out: W, s: &State, size: (u16, u16)) -> Result<usize>
where
    W: io::Write,
{
    let h = size.1 as usize - 2;
    let offset = if s.cursor.y < h / 2 || s.text.len() <= h {
        0
    } else if s.cursor.y - h / 2 + h <= s.text.len() {
        s.cursor.y - h / 2
    } else {
        s.text.len() - h
    };
    let selection = s.selection();
    for (y, line) in s.text[offset..usize::min(offset + h, s.text.len())]
        .iter()
        .enumerate()
    {
        queue!(out, cursor::MoveTo(0, y as u16))?;
        let mut w = 0;
        for (x, c) in line.0.char_indices().chain(iter::once((line.0.len(), ' '))) {
            let p = Point { x, y: y + offset };
            w += c.width().unwrap_or(0) as u16;
            if w >= size.0 {
                // TODO wrap or scroll
                break;
            }
            if line.1.comment_indices.contains(&x) {
                queue!(out, style::SetForegroundColor(Color::DarkRed))?;
            }
            if line.1.match_indices.contains(&x) {
                queue!(out, style::SetBackgroundColor(Color::Red))?;
            }
            if selection
                .map(|(start, end)| p >= start.into() && p < end.into())
                .unwrap_or(false)
            {
                queue!(out, style::SetBackgroundColor(Color::Grey))?;
            }
            queue!(out, style::Print(c), style::ResetColor)?;
        }
        if w < size.0 {
            queue!(out, terminal::Clear(ClearType::UntilNewLine))?;
        }
    }
    Ok(offset)
}

fn draw_status<W>(mut out: W, s: &State, size: (u16, u16)) -> Result<()>
where
    W: io::Write,
{
    queue!(
        out,
        cursor::MoveTo(0, size.1 - 2),
        style::SetBackgroundColor(Color::Grey),
        style::Print(format!(
            "{:6} {:<4$} {:4}:{:<3}",
            &s.mode,
            &s.file,
            s.cursor.y + 1,
            s.cursor.x + 1,
            size.0 as usize - 16,
        )),
        style::ResetColor,
        cursor::MoveTo(0, size.1 - 1),
    )?;
    Ok(())
}

fn draw_search<W>(mut out: W, s: &State, size: (u16, u16)) -> Result<()>
where
    W: io::Write,
{
    queue!(out, cursor::MoveTo(0, size.1 - 1))?;
    match &s.search {
        Some(Ok(re)) => {
            queue!(out, style::Print('/'), style::Print(re))?;
        }
        Some(Err(regex::Error::Syntax(msg))) => {
            queue!(out, style::Print("! "), style::Print(msg))?;
        }
        Some(Err(err)) => {
            bail!("Unhandled error parsing regex for search: {}", err);
        }
        None => (),
    }
    queue!(out, terminal::Clear(ClearType::UntilNewLine))?;
    Ok(())
}

pub fn draw<W>(mut out: W, s: &State, size: (u16, u16)) -> Result<()>
where
    W: io::Write,
{
    queue!(out, cursor::Hide)?;
    let offset = draw_text(&mut out, s, size)?;
    draw_status(&mut out, s, size)?;
    draw_search(&mut out, s, size)?;
    queue!(
        out,
        cursor::MoveTo(s.cursor_width() as u16, (s.cursor.y - offset) as u16),
        cursor::Show,
    )?;
    out.flush()?;
    Ok(())
}
