pub mod entry;
pub mod ops;
pub mod watcher;

pub use entry::{read_dir, Entry, EntryKind};
pub use ops::{
    copy_entries, delete_entries, move_entries, rename_entry,
    create_file, create_dir, ConflictResolution, Conflict, OpResult,
};
pub use watcher::FsWatcher;
