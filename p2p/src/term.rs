use std::io::{stdout, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{self, Attribute, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};

pub type TermError = std::io::Error;

pub fn reset() -> Result<(), TermError> {
    let mut stdout = stdout();
    execute!(stdout, cursor::MoveTo(0, 2))?;
    execute!(stdout, terminal::Clear(ClearType::All))?;
    Ok(())
}

pub fn print_title(title: &str, color: style::Color) -> Result<(), TermError> {
    let mut stdout = stdout();
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 2),
        SetForegroundColor(color),
        SetAttribute(Attribute::Bold),
        SetAttribute(Attribute::Underlined),
        Print(title),
        SetAttribute(Attribute::Reset)
    )?;

    Ok(())
}

pub fn space() -> Result<(), TermError> {
    let mut stdout = stdout();
    write!(stdout, "\r\n",)?;
    Ok(())
}

pub fn print(value: &str, color: style::Color) -> Result<(), TermError> {
    let mut stdout = stdout();
    write!(
        stdout,
        "\r{} {}\r{}",
        SetForegroundColor(color),
        value,
        ResetColor
    )?;

    Ok(())
}

pub fn println(value: &str, color: style::Color) -> Result<(), TermError> {
    let mut stdout = stdout();

    write!(
        stdout,
        "\r{} {}\r\n{}",
        SetForegroundColor(color),
        value,
        ResetColor
    )?;

    Ok(())
}

pub fn hide_cursor(hide: bool) -> Result<(), TermError> {
    let mut stdout = stdout();

    if hide {
        execute!(stdout, cursor::Hide)?;
        return Ok(());
    }

    execute!(stdout, cursor::Show)?;
    Ok(())
}

pub fn move_cursor(x: u16, y: u16) -> Result<(), TermError> {
    let mut stdout = stdout();
    execute!(stdout, cursor::MoveTo(x, y))?;
    Ok(())
}

pub fn wait_for_enter() -> Result<(), TermError> {
    loop {
        if let Event::Key(KeyEvent {
            code: KeyCode::Enter,
            ..
        }) = event::read()?
        {
            break;
        }
    }

    Ok(())
}

pub fn menu(title: String, options: Vec<String>) -> Result<usize, TermError> {
    let mut stdout = stdout();
    let mut selected = 0;

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;
    execute!(stdout, terminal::Clear(ClearType::All))?;

    loop {
        execute!(stdout, cursor::MoveTo(0, 2))?;
        print_title(title.as_str(), style::Color::Cyan)?;

        execute!(stdout, cursor::MoveTo(0, 4))?;
        for (i, option) in options.iter().enumerate() {
            if i == selected {
                write!(
                    stdout,
                    "{}> {}\r\n{}",
                    SetForegroundColor(style::Color::DarkMagenta),
                    option,
                    ResetColor
                )?;
            } else {
                write!(stdout, "  {}\r\n", option)?;
            }
        }

        space()?;
        println("## Accept - <Enter> || Exit - <Q>", style::Color::Yellow)?;

        stdout.flush()?;

        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected < options.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Enter => break,
                KeyCode::Char('q') => {
                    selected = usize::MAX;
                    break;
                }
                _ => {}
            }
        }
    }

    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(selected)
}
