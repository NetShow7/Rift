use crate::config::LayoutMode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct LayoutAreas {
    pub panes: Vec<Rect>,   // 1 (single), 2 (dual/miller active+parent), or 3 (miller all)
    pub statusbar: Rect,
    pub preview: Option<Rect>,
}

pub fn compute_layout(
    area: Rect,
    mode: &LayoutMode,
    show_preview: bool,
) -> LayoutAreas {
    // Reserve bottom row for status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main = chunks[0];
    let statusbar = chunks[1];

    match mode {
        LayoutMode::Single => {
            if show_preview {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(main);
                LayoutAreas {
                    panes: vec![cols[0]],
                    statusbar,
                    preview: Some(cols[1]),
                }
            } else {
                LayoutAreas {
                    panes: vec![main],
                    statusbar,
                    preview: None,
                }
            }
        }

        LayoutMode::Dual => {
            let cols = if show_preview {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(35),
                        Constraint::Percentage(35),
                        Constraint::Percentage(30),
                    ])
                    .split(main)
            } else {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(main)
            };
            if show_preview {
                LayoutAreas {
                    panes: vec![cols[0], cols[1]],
                    statusbar,
                    preview: Some(cols[2]),
                }
            } else {
                LayoutAreas {
                    panes: vec![cols[0], cols[1]],
                    statusbar,
                    preview: None,
                }
            }
        }

        LayoutMode::Miller => {
            if show_preview {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(25),
                        Constraint::Percentage(40),
                        Constraint::Percentage(35),
                    ])
                    .split(main);
                LayoutAreas {
                    panes: vec![cols[0], cols[1]],
                    statusbar,
                    preview: Some(cols[2]),
                }
            } else {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(35),
                        Constraint::Percentage(65),
                    ])
                    .split(main);
                LayoutAreas {
                    panes: vec![cols[0], cols[1]],
                    statusbar,
                    preview: None,
                }
            }
        }
    }
}
