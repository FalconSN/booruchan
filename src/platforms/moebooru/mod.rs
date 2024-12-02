mod moebooru;
pub use moebooru::Moebooru;
pub(super) use types::{Params, Post, Posts, Status};

mod types {
    use crate::worker::DbEntry;
    use serde::{Deserialize, Serialize};

    pub(crate) type Posts = Vec<Option<Post>>;

    #[derive(Debug, Serialize)]
    pub struct Params<'p> {
        pub api_version: u8,
        pub include_tags: u8,
        pub limit: u8,
        pub page: u64,
        pub tags: &'p str,
    }

    impl<'p> Default for Params<'p> {
        fn default() -> Self {
            Self {
                api_version: 2,
                include_tags: 1,
                limit: 100,
                page: 0,
                tags: "",
            }
        }
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "lowercase")]
    pub(crate) enum Status {
        Active,
        Pending,
        Flagged,
        Deleted,
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    pub struct FlagDetail {
        pub post_id: i64,
        pub reason: String,
        pub created_at: String,
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    pub struct Post {
        pub id: i64,
        pub tags: String,
        pub created_at: i64,
        pub updated_at: Option<i64>,
        pub creator_id: i64,
        pub approver_id: Option<i64>,
        pub author: String,
        pub change: i64,
        pub source: String,
        pub score: i64,
        pub md5: String,
        pub file_size: i64,
        pub file_ext: Option<String>,
        #[serde(default)]
        pub file_url: String,
        pub is_shown_in_index: bool,
        pub preview_url: String,
        pub preview_width: u32,
        pub preview_height: u32,
        pub actual_preview_width: u32,
        pub actual_preview_height: u32,
        #[serde(default)]
        pub sample_url: String,
        pub sample_width: u32,
        pub sample_height: u32,
        pub sample_file_size: u64,
        #[serde(default)]
        pub jpeg_url: String,
        pub jpeg_width: u64,
        pub jpeg_height: u64,
        pub jpeg_file_size: u64,
        pub rating: String,
        pub is_rating_locked: Option<bool>,
        pub has_children: bool,
        pub parent_id: Option<u64>,
        pub status: Status,
        pub is_pending: Option<bool>,
        pub width: u64,
        pub height: u64,
        pub is_held: bool,
        pub frames_pending_string: String,
        pub frames_pending: Vec<String>,
        pub frames_string: String,
        pub frames: Vec<String>,
        pub is_note_locked: Option<bool>,
        pub last_noted_at: Option<u64>,
        pub last_commented_at: Option<u64>,
        pub flag_detail: Option<FlagDetail>,
        #[serde(skip, default = "bool::default")]
        pub is_duplicate: bool,
        #[serde(skip, default = "Option::default")]
        pub duplicate_entry: Option<DbEntry>,
    }
}
