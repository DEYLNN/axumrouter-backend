// RTK filter constants + re-exports from individual filter modules.
// Filters split into src/services/rtk/filters/<name>.rs for maintainability.

pub use crate::services::rtk::filters::{
    git_diff::git_diff,
    git_status::git_status,
    grep::grep,
    find::find,
    dedup_log::dedup_log,
    smart_truncate::smart_truncate,
    read_numbered::read_numbered,
    build_output::build_output,
    tree::tree,
    ls::ls,
    search_list::search_list,
};

pub const RAW_CAP: usize = 10 * 1024 * 1024;
pub const MIN_COMPRESS_SIZE: usize = 500;
pub const DETECT_WINDOW: usize = 1024;
pub const GIT_DIFF_HUNK_MAX_LINES: usize = 100;
pub const DEDUP_LINE_MAX: usize = 2000;
pub const GREP_PER_FILE_MAX: usize = 10;
pub const FIND_PER_DIR_MAX: usize = 10;
pub const FIND_TOTAL_DIR_MAX: usize = 20;
pub const STATUS_MAX_FILES: usize = 10;
pub const STATUS_MAX_UNTRACKED: usize = 10;
pub const LS_EXT_SUMMARY_TOP: usize = 5;
pub const TREE_MAX_LINES: usize = 200;
pub const SEARCH_LIST_PER_DIR_MAX: usize = 10;
pub const SEARCH_LIST_TOTAL_DIR_MAX: usize = 20;
pub const SMART_TRUNCATE_HEAD: usize = 120;
pub const SMART_TRUNCATE_TAIL: usize = 60;
pub const SMART_TRUNCATE_MIN_LINES: usize = 250;
pub const READ_NUMBERED_MIN_HIT_RATIO: f64 = 0.7;
pub const MAX_RESULT_LINES: usize = 500;
