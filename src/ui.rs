use {
    crate::state::{Point, State},
    anyhow::Result,
    crossterm::{
        cursor, queue,
        style::{self, Color},
        terminal,
    },
    lazy_static::lazy_static,
    regex::Regex,
    std::{io, iter},
    unicode_width::UnicodeWidthChar,
};

lazy_static! {
    static ref COMMENT: Regex = Regex::new("//").unwrap();
}

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
        let comment = COMMENT.find(line).map(|m| m.start());
        queue!(out, cursor::MoveTo(0, y as u16))?;
        let mut w = 0;
        for (x, c) in line.char_indices().chain(iter::once((line.len(), ' '))) {
            let p = Point { x, y: y + offset };
            w += c.width().unwrap_or(0) as u16;
            if w >= size.0 {
                // TODO wrap or scroll
                break;
            }
            if comment.map(|start| x >= start).unwrap_or(false) {
                queue!(out, style::SetForegroundColor(Color::DarkRed))?;
            }
            if selection
                .map(|(start, end)| p >= start.into() && p < end.into())
                .unwrap_or(false)
            {
                queue!(out, style::SetBackgroundColor(Color::Grey))?;
            }
            queue!(out, style::Print(c), style::ResetColor)?;
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
        style::Print(&s.file),
        style::Print(format!(
            "{:>1$}",
            format!("{},{}", s.cursor.x + 1, s.cursor.y + 1),
            size.0 as usize - s.file.len()
        )),
        style::ResetColor,
        cursor::MoveTo(0, size.1 - 1),
        style::Print(&s.mode),
    )?;
    Ok(())
}

pub fn draw<W>(mut out: W, s: &State, size: (u16, u16)) -> Result<()>
where
    W: io::Write,
{
    queue!(out, terminal::Clear(terminal::ClearType::All))?;
    let offset = draw_text(&mut out, s, size)?;
    draw_status(&mut out, s, size)?;
    queue!(
        out,
        cursor::MoveTo(s.cursor_width() as u16, (s.cursor.y - offset) as u16)
    )?;
    out.flush()?;
    Ok(())
}
