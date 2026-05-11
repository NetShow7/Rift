use crate::fs::Conflict;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

/// All modal dialogs the app can show.
#[derive(Debug, Clone)]
pub enum Modal {
    /// Conflict during copy/move: show options to user.
    Conflict {
        conflict: ConflictDialogState,
    },
    /// Simple yes/no confirmation (e.g. delete).
    Confirm {
        title: String,
        message: String,
        selected: ConfirmChoice,
    },
    /// Single-line text input (rename, new file/dir).
    Input {
        title: String,
        prompt: String,
        value: String,
        cursor: usize,
    },
    /// Operation summary after batch op completes.
    Summary {
        title: String,
        lines: Vec<String>,
        scroll: usize,
    },
    /// Help overlay.
    Help { scroll: usize },
}

#[derive(Debug, Clone)]
pub struct ConflictDialogState {
    pub conflict: Conflict,
    pub selected: ConflictChoice,
    /// If user picks Rename, they type here.
    pub rename_input: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictChoice {
    Skip,
    Overwrite,
    Rename,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmChoice {
    Yes,
    No,
}

impl Modal {
    pub fn conflict(conflict: Conflict) -> Self {
        Self::Conflict {
            conflict: ConflictDialogState {
                conflict,
                selected: ConflictChoice::Skip,
                rename_input: None,
            },
        }
    }

    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Confirm {
            title: title.into(),
            message: message.into(),
            selected: ConfirmChoice::No,
        }
    }

    pub fn input(title: impl Into<String>, prompt: impl Into<String>, prefill: &str) -> Self {
        let value = prefill.to_string();
        let cursor = value.len();
        Self::Input {
            title: title.into(),
            prompt: prompt.into(),
            value,
            cursor,
        }
    }
}

/// Centred overlay rect.
pub fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(area.width * percent_x / 100)) / 2;
    let w = area.width * percent_x / 100;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect { x, y, width: w, height: height.min(area.height) }
}

pub fn draw_modal(frame: &mut Frame, modal: &Modal, area: Rect) {
    match modal {
        Modal::Conflict { conflict } => draw_conflict(frame, conflict, area),
        Modal::Confirm { title, message, selected } => {
            draw_confirm(frame, title, message, selected, area)
        }
        Modal::Input { title, prompt, value, cursor } => {
            draw_input(frame, title, prompt, value, *cursor, area)
        }
        Modal::Summary { title, lines, scroll } => {
            draw_summary(frame, title, lines, *scroll, area)
        }
        Modal::Help { scroll } => draw_help(frame, *scroll, area),
    }
}

fn draw_conflict(frame: &mut Frame, state: &ConflictDialogState, area: Rect) {
    let rect = centered_rect(60, 12, area);
    frame.render_widget(Clear, rect);

    let src_name = state.conflict.src.file_name()
        .unwrap_or_default().to_string_lossy();
    let dst_name = state.conflict.dst.display().to_string();

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Conflict ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(187, 154, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let choices = [
        (ConflictChoice::Skip,      "[S]kip"),
        (ConflictChoice::Overwrite, "[O]verwrite"),
        (ConflictChoice::Rename,    "[R]ename"),
        (ConflictChoice::Abort,     "[A]bort"),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(format!("\"{}\" already exists at:", src_name))
            .alignment(Alignment::Center),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(dst_name)
            .style(Style::default().fg(Color::Rgb(224, 175, 104)))
            .alignment(Alignment::Center),
        chunks[1],
    );

    let button_line: Line = choices
        .iter()
        .map(|(choice, label)| {
            let style = if *choice == state.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(187, 154, 247))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(169, 177, 214))
            };
            Span::styled(format!(" {} ", label), style)
        })
        .collect::<Vec<_>>()
        .into();

    frame.render_widget(
        Paragraph::new(button_line).alignment(Alignment::Center),
        chunks[3],
    );

    if let Some(ref rename) = state.rename_input {
        frame.render_widget(
            Paragraph::new(format!("New name: {}_", rename))
                .style(Style::default().fg(Color::Rgb(158, 206, 106))),
            chunks[4],
        );
    }
}

fn draw_confirm(
    frame: &mut Frame,
    title: &str,
    message: &str,
    selected: &ConfirmChoice,
    area: Rect,
) {
    let rect = centered_rect(50, 8, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(247, 118, 142)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(2)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(message).alignment(Alignment::Center),
        chunks[0],
    );

    let yes_style = if *selected == ConfirmChoice::Yes {
        Style::default().fg(Color::Black).bg(Color::Rgb(247, 118, 142)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(169, 177, 214))
    };
    let no_style = if *selected == ConfirmChoice::No {
        Style::default().fg(Color::Black).bg(Color::Rgb(169, 177, 214)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(169, 177, 214))
    };

    let line = Line::from(vec![
        Span::styled("  [Y]es  ", yes_style),
        Span::raw("  "),
        Span::styled("  [N]o  ", no_style),
    ]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), chunks[1]);
}

fn draw_input(
    frame: &mut Frame,
    title: &str,
    prompt: &str,
    value: &str,
    _cursor: usize,
    area: Rect,
) {
    let rect = centered_rect(55, 7, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(122, 162, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(Paragraph::new(prompt), chunks[0]);
    frame.render_widget(
        Paragraph::new(format!("{}_", value))
            .style(Style::default().fg(Color::Rgb(192, 202, 245))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("Enter to confirm • Esc to cancel")
            .style(Style::default().fg(Color::Rgb(86, 95, 137)))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_summary(frame: &mut Frame, title: &str, lines: &[String], scroll: usize, area: Rect) {
    let rect = centered_rect(65, 20, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(158, 206, 106)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let visible: Vec<Line> = lines
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|l| Line::raw(l.clone()))
        .collect();

    frame.render_widget(Paragraph::new(visible), inner);
}

fn draw_help(frame: &mut Frame, scroll: usize, area: Rect) {
    let rect = centered_rect(70, 30, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Help — ? to close ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(122, 162, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let entries = vec![
        ("Navigation", vec![
            ("↑/↓ or k/j", "Move cursor"),
            ("←/→ or h/l", "Go to parent / open"),
            ("Enter",       "Open entry"),
            ("PgUp/PgDn",   "Page scroll"),
            ("g/G",         "Go to top / bottom"),
        ]),
        ("Selection", vec![
            ("Space",   "Toggle selection"),
            ("Ctrl+A",  "Select all"),
            ("Escape",  "Clear selection"),
        ]),
        ("File operations", vec![
            ("Ctrl+C", "Copy"),
            ("Ctrl+X", "Cut"),
            ("Ctrl+V", "Paste"),
            ("Delete", "Delete"),
            ("F2",     "Rename"),
            ("Ctrl+N", "New file"),
            ("Ctrl+Shift+N", "New directory"),
        ]),
        ("View", vec![
            ("Ctrl+H", "Toggle hidden files"),
            ("Ctrl+P", "Toggle preview"),
            ("Tab",    "Cycle layout"),
            ("r",      "Refresh"),
        ]),
        ("App", vec![
            ("/",     "Search"),
            ("f",     "Filter"),
            ("q",     "Quit"),
        ]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (section, bindings) in &entries {
        lines.push(Line::from(Span::styled(
            *section,
            Style::default()
                .fg(Color::Rgb(187, 154, 247))
                .add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:20}", key), Style::default().fg(Color::Rgb(224, 175, 104))),
                Span::raw(*desc),
            ]));
        }
        lines.push(Line::raw(""));
    }

    let visible: Vec<Line> = lines.into_iter().skip(scroll).take(inner.height as usize).collect();
    frame.render_widget(Paragraph::new(visible), inner);
}
