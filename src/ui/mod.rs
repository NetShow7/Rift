pub mod layout;
pub mod modal;
pub mod pane;
pub mod preview;
pub mod sidebar;
pub mod syntax_highlighter;
pub mod statusbar;

pub use layout::{compute_layout, LayoutAreas};
pub use modal::{draw_modal, ConflictChoice, ConfirmChoice, InputIntent, Modal};
pub use pane::Pane;
pub use preview::{draw_preview, PreviewCache, PreviewContent};
pub use sidebar::{render_sidebar, AnimState, SidebarSection, SidebarState};
pub use statusbar::StatusBar;
pub use syntax_highlighter::{highlight_text, HighlightedLine};
