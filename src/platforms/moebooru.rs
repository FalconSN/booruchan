// std imports
use std::path::PathBuf;

// crate imports
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, oneshot},
    time::{sleep, Duration},
};

// local imports
use crate::{
    config::{Compress, PlatformConfig},
    consts::tag_types::*,
    fmt::Keywords,
    rclone,
    statics::HOME,
    utils,
    worker::{DbEntry, ImageRequest, Insert, Operation, Select},
    Downloader,
};

pub struct Moebooru {
    platform: &'static str,
    root: &'static str,
    config: PlatformConfig,
    worker: mpsc::Sender<Operation>,
    client: Client,
    timer: Timer,
}

impl Moebooru {
    pub fn new(
        platform: &'static str,
        root: &'static str,
        config: PlatformConfig,
        worker: mpsc::Sender<Operation>,
        client: Client,
    ) -> Self {
        let timer = Timer {
            retry_sleep: Duration::from_secs_f32(config.retry_sleep),
            timeout: Duration::from_secs_f32(config.timeout),
            sleep: Duration::from_secs_f32(config.sleep),
        };
        Self {
            platform,
            root,
            config,
            worker,
            client,
            timer,
        }
    }

    async fn filter(&self, posts: &mut Posts) {
        for post in posts.iter_mut() {
            if self.config.blacklist.len() > 0 {
                if post.as_ref().is_some_and(|p| {
                    self.config
                        .blacklist
                        .iter()
                        .map(|t| t.as_str())
                        .any(|tag| p.tags.contains(tag))
                }) {
                    *post = None;
                    continue;
                }
            }
            match post {
                Some(_post) => {
                    let (comm_send, comm_recv): (
                        oneshot::Sender<Option<DbEntry>>,
                        oneshot::Receiver<Option<DbEntry>>,
                    ) = oneshot::channel();
                    let op = Operation::Select(Select {
                        platform: self.platform,
                        id: _post.id,
                        sender: comm_send,
                    });
                    self.worker.send(op).await.unwrap();
                    let recv = comm_recv.await.unwrap();
                    match recv {
                        Some(db) => {
                            if db.id == _post.id {
                                println!("duplicate: {}", _post.id);
                                _post.is_duplicate = true;
                                _post.duplicate_entry = Some(db);
                            }
                        }
                        None => (),
                    }
                }
                None => (),
            }
        }
    }

    async fn to_keywords<'kw>(&'kw self, post: &'kw Post, tag_map: &'kw TagMap) -> Keywords<'kw> {
        let mut keywords = Keywords {
            platform: self.platform,
            id: post.id,
            tags: post.tags.as_str(),
            source: post.source.as_str(),
            md5: post.md5.as_str(),
            file_size: post.file_size,
            file_ext: if post.file_ext.is_some() {
                post.file_ext.as_ref().unwrap().as_str()
            } else {
                post.file_url.rsplit_once('.').unwrap().1
            },
            rating: post.rating.as_str(),
            general: Vec::new(),
            character: Vec::new(),
            copyright: Vec::new(),
            artist: Vec::new(),
            metadata: Vec::new(),
            circle: Vec::new(),
            faults: Vec::new(),
            style: Vec::new(),
        };
        for tag in post.tags.split_whitespace() {
            match tag_map.get(tag) {
                Some(_type) => match _type.as_str() {
                    GENERAL => keywords.general.push(tag),
                    CHARACTER => keywords.character.push(tag),
                    COPYRIGHT => keywords.copyright.push(tag),
                    ARTIST => keywords.artist.push(tag),
                    METADATA => keywords.metadata.push(tag),
                    CIRCLE => keywords.circle.push(tag),
                    FAULTS => keywords.faults.push(tag),
                    STYLE => keywords.style.push(tag),
                    _ => panic!(
                        "unexpected tag type for {id} in {platform}: {_type}",
                        id = post.id,
                        platform = self.platform,
                    ),
                },
                None => panic!(
                    "couldn't find tag in tag_map: {tag}\npost id: {id}, platform: {platform}",
                    id = post.id,
                    platform = self.platform
                ),
            }
        }
        return keywords;
    }

    async fn handle_duplicate<'dup>(&self, db_entry: DbEntry, duplicate_entry: &DbEntry) -> () {
        if db_entry.path != duplicate_entry.path {
            if self.config.to_cloud {
                let cloud = self.config.cloud.as_str();
                rclone::moveto(
                    format!("{cloud}:{src}", src = duplicate_entry.path.as_str(),),
                    format!("{cloud}:{dest}", dest = db_entry.path.as_str(),),
                    || async {},
                )
                .await;
            } else {
                utils::mvf(
                    duplicate_entry.path.as_str(),
                    db_entry.path.as_str(),
                    || async {},
                )
                .await;
            }
        }
        if let (Some(old_path), Some(new_path)) = (
            duplicate_entry.compress_path.as_ref(),
            db_entry.compress_path.as_ref(),
        ) {
            if self.config.to_cloud {
                let cloud = self.config.cloud.as_str();
                rclone::moveto(
                    format!("{cloud}:{old_path}"),
                    format!("{cloud}:{new_path}"),
                    || async {},
                )
                .await;
            } else {
                utils::mvf(old_path, new_path, || async {}).await;
            }
        }
        self.worker
            .send(Operation::Insert(Insert {
                platform: self.platform,
                entry: db_entry,
            }))
            .await
            .unwrap();
    }

    async fn handle_compression(
        &self,
        file: &PathBuf,
        compress: &Compress,
        keywords: &Keywords<'_>,
        db_entry: &mut DbEntry,
    ) {
        let path = [compress.target_dir.as_str(), compress.filename.as_str()]
            .iter()
            .map(|s| keywords.format(s))
            .collect::<Vec<String>>();
        let (local_send, local_recv): (
            oneshot::Sender<Option<PathBuf>>,
            oneshot::Receiver<Option<PathBuf>>,
        ) = oneshot::channel();
        self.worker
            .send(Operation::Image(ImageRequest {
                src: file.clone(),
                dest: path.clone(),
                size: compress.size,
                fallback: match self.config.to_cloud {
                    true => Some(HOME.to_string()),
                    false => None,
                },
                response_channel: local_send,
            }))
            .await
            .unwrap();
        let resp = local_recv.await.unwrap();
        match resp {
            Some(file) => {
                if self.config.to_cloud {
                    let _dest_path = path.join("/");
                    if rclone::copyto(
                        file.to_str().unwrap(),
                        format!("{}:{}", self.config.cloud.as_str(), _dest_path.as_str()).as_str(),
                        //_dest_path.as_str(),
                        self.config.delete,
                        || async {},
                    )
                    .await
                    {
                        db_entry.compress_path = Some(_dest_path);
                    }
                } else {
                    db_entry.compress_path = Some(file.to_str().unwrap().to_string());
                }
            }
            None => (),
        }
    }

    #[allow(unused_assignments)]
    async fn post_task(&self, post: Post, tag_map: &TagMap) {
        let mut is_success = false;
        let keywords = self.to_keywords(&post, tag_map).await;
        let target_dir: String = keywords.format(self.config.target_dir.as_str());
        let filename: String = keywords.format(self.config.filename.as_str());
        let full_path_vec = Vec::from([target_dir.as_str(), filename.as_str()]);
        let full_path = full_path_vec.join("/");
        let mut db_entry = DbEntry {
            id: post.id,
            md5: keywords.md5.to_string(),
            source: if post.source.is_empty() {
                None
            } else {
                Some(post.source.clone())
            },
            tags: if post.tags.is_empty() {
                None
            } else {
                Some(post.tags.clone())
            },
            path: full_path.clone(),
            compress_path: None,
        };
        if post.is_duplicate {
            match post.duplicate_entry {
                Some(ref duplicate_entry) => {
                    if db_entry != *duplicate_entry {
                        return self.handle_duplicate(db_entry.clone(), duplicate_entry).await;
                    }

                },
                None => panic!("unexpected event: is_duplicate is true but encountered None for db_entry\npost: {:?}", post),
            }
        }

        let downloaded = Downloader::new(
            self.client.clone(),
            post.file_url.as_str(),
            full_path_vec,
            Some(HOME.as_str()),
            self.timer.timeout,
            self.config.retries,
            self.timer.retry_sleep,
        )
        .download()
        .await;
        match downloaded {
            Some(file) => {
                match self.config.compress {
                    Some(ref compress) => {
                        self.handle_compression(&file, compress, &keywords, &mut db_entry)
                            .await
                    }
                    None => (),
                }
                if self.config.to_cloud {
                    is_success = rclone::copyto(
                        file.to_str().unwrap(),
                        format!("{}:{}", self.config.cloud.as_str(), full_path.as_str()).as_str(),
                        self.config.delete,
                        || async {},
                    )
                    .await;
                } else {
                    is_success = true;
                }
            }
            None => return,
        }

        if is_success {
            self.worker
                .send(Operation::Insert(Insert {
                    platform: self.platform,
                    entry: db_entry,
                }))
                .await
                .unwrap();
        }
    }

    async fn tag_task<T: Serialize>(&self, params: &T, client: &Client) -> Option<(Posts, TagMap)> {
        loop {
            let mut response: serde_json::Value = {
                loop {
                    match client.get(self.root).query(params).send().await {
                        Ok(r) => match r.json().await {
                            Ok(j) => break j,
                            Err(e) => {
                                eprintln!("{e:?}");
                                continue;
                            }
                        },
                        Err(e) => {
                            eprintln!("{e:?}");
                            continue;
                        }
                    }
                }
            };
            let mut posts: Posts = match serde_json::from_value(response["posts"].take()) {
                Ok(p) => p,
                Err(e) => panic!("{e:?}\nplatform: {}", self.platform),
            };
            let posts_len = posts.len();
            if posts_len == 0 {
                return None;
            }
            if self.config.skip {
                self.filter(&mut posts).await;
            }
            let tag_map: TagMap = serde_json::from_value(response["tags"].take()).unwrap();
            return Some((posts, tag_map));
        }
    }

    pub async fn main(self) {
        let mut params = Params::default();
        for tag in self.config.tags.iter().map(|t| t.as_str()) {
            println!("{}: {}", self.platform, tag);
            (params.page, params.tags) = (0, tag);
            while let Some((posts, tag_map)) = {
                params.page += 1;
                self.tag_task(&params, &self.client).await
            } {
                for post in posts {
                    match post {
                        Some(p) if p.status != Status::Deleted => {
                            sleep(self.timer.sleep).await;
                            self.post_task(p, &tag_map).await;
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

type TagMap = std::collections::BTreeMap<String, String>;
type Posts = Vec<Option<Post>>;

struct Timer {
    retry_sleep: Duration,
    timeout: Duration,
    sleep: Duration,
}

#[derive(Debug, Serialize)]
struct Params<'p> {
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
enum Status {
    Active,
    Pending,
    Flagged,
    Deleted,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FlagDetail {
    pub post_id: i64,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Post {
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
