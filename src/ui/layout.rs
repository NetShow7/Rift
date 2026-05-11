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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn test_area() -> Rect {
        Rect { x: 0, y: 0, width: 100, height: 50 }
    }

    #[test]
    fn single_no_preview() {
        let l = compute_layout(test_area(), &LayoutMode::Single, false);
        assert_eq!(l.panes.len(), 1);
        assert!(l.preview.is_none());
        assert_eq!(l.panes[0].height, 49);
        assert_eq!(l.panes[0].width, 100);
        assert_eq!(l.statusbar.y, 49);
        assert_eq!(l.statusbar.height, 1);
    }

    #[test]
    fn single_with_preview() {
        let l = compute_layout(test_area(), &LayoutMode::Single, true);
        assert_eq!(l.panes.len(), 1);
        assert!(l.preview.is_some());
        assert_eq!(l.preview.unwrap().width, 40);
        assert_eq!(l.panes[0].width, 60);
    }

    #[test]
    fn dual_no_preview() {
        let l = compute_layout(test_area(), &LayoutMode::Dual, false);
        assert_eq!(l.panes.len(), 2);
        assert!(l.preview.is_none());
        assert_eq!(l.panes[0].width, 50);
        assert_eq!(l.panes[1].width, 50);
    }

    #[test]
    fn dual_with_preview() {
        let l = compute_layout(test_area(), &LayoutMode::Dual, true);
        assert_eq!(l.panes.len(), 2);
        assert!(l.preview.is_some());
        assert_eq!(l.preview.unwrap().width, 30);
        assert_eq!(l.panes[0].width, 35);
        assert_eq!(l.panes[1].width, 35);
    }

    #[test]
    fn miller_no_preview() {
        let l = compute_layout(test_area(), &LayoutMode::Miller, false);
        assert_eq!(l.panes.len(), 2);
        assert!(l.preview.is_none());
        assert_eq!(l.panes[0].width, 35);
        assert_eq!(l.panes[1].width, 65);
    }

    #[test]
    fn miller_with_preview() {
        let l = compute_layout(test_area(), &LayoutMode::Miller, true);
        assert_eq!(l.panes.len(), 2);
        assert!(l.preview.is_some());
        assert_eq!(l.preview.unwrap().width, 35);
        assert_eq!(l.panes[0].width, 25);
        assert_eq!(l.panes[1].width, 40);
    }

    #[test]
    fn statusbar_always_at_bottom() {
        for (mode, preview) in &[
            (LayoutMode::Single, false),
            (LayoutMode::Single, true),
            (LayoutMode::Dual, false),
            (LayoutMode::Dual, true),
            (LayoutMode::Miller, false),
            (LayoutMode::Miller, true),
        ] {
            let l = compute_layout(test_area(), mode, *preview);
            assert_eq!(l.statusbar.y, 49, "Statusbar y for {:?} preview={}", mode, preview);
            assert_eq!(l.statusbar.height, 1);
        }
    }

    #[test]
    fn layout_uses_full_width() {
        let l = compute_layout(test_area(), &LayoutMode::Single, false);
        assert_eq!(l.panes[0].x, 0);
        assert_eq!(l.statusbar.x, 0);
        assert_eq!(l.statusbar.width, 100);
    }

    #[test]
    fn layout_small_area() {
        let small = Rect { x: 0, y: 0, width: 20, height: 5 };
        let l = compute_layout(small, &LayoutMode::Dual, true);
        assert_eq!(l.panes.len(), 2);
        assert!(l.preview.is_some());
        assert_eq!(l.statusbar.height, 1);
    }
}
