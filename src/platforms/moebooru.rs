// std imports
use std::path::PathBuf;

// crate imports
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};

// local imports
use super::base::PlatformConfig;
use crate::{
    downloader::download,
    rclone, utils,
    worker::{DbEntry, DbEntryOwned, ImageRequest, Insert, Operation, Select},
    KeywordMap, HOME,
};

fn _false() -> bool {
    false
}

fn _none() -> Option<DbEntryOwned> {
    None
}

const TAGS: &str = "tags";
const POSTS: &str = "posts";
const GENERAL: &str = "general";
const CHARACTER: &str = "character";
const ARTIST: &str = "artist";
const COPYRIGHT: &str = "copyright";
const METADATA: &str = "metadata";
const CIRCLE: &str = "circle";
const FAULTS: &str = "faults";

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct MoePost {
    id: i64,
    tags: String,
    created_at: i64,
    updated_at: i64,
    creator_id: i64,
    approver_id: Option<i64>,
    author: String,
    change: i64,
    source: String,
    score: i64,
    md5: String,
    file_size: i64,
    file_ext: String,
    file_url: String,
    is_shown_in_index: bool,
    preview_url: String,
    preview_width: u32,
    preview_height: u32,
    actual_preview_width: u32,
    actual_preview_height: u32,
    sample_url: String,
    sample_width: u32,
    sample_height: u32,
    sample_file_size: u64,
    jpeg_url: String,
    jpeg_width: u64,
    jpeg_height: u64,
    jpeg_file_size: u64,
    rating: String,
    is_rating_locked: bool,
    has_children: bool,
    parent_id: Option<u64>,
    status: String,
    is_pending: bool,
    width: u64,
    height: u64,
    is_held: bool,
    frames_pending_string: String,
    frames_pending: Vec<String>,
    frames_string: String,
    frames: Vec<String>,
    is_note_locked: bool,
    last_noted_at: u64,
    last_commented_at: u64,
    #[serde(skip, default = "_false")]
    is_duplicate: bool,
    #[serde(skip, default = "_none")]
    db_entry: Option<DbEntryOwned>,
}

type Posts = Vec<Option<MoePost>>;
//type TagMap = BTreeMap<String, Vec<String>>;

struct TagMap {
    general: Vec<String>,
    character: Vec<String>,
    copyright: Vec<String>,
    artist: Vec<String>,
    metadata: Vec<String>,
    circle: Vec<String>,
    faults: Vec<String>,
}

impl TagMap {
    fn new() -> Self {
        Self {
            general: Vec::new(),
            character: Vec::new(),
            copyright: Vec::new(),
            artist: Vec::new(),
            metadata: Vec::new(),
            circle: Vec::new(),
            faults: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct Params<'p> {
    pub api_version: u8,
    pub include_tags: u8,
    pub limit: u8,
    pub page: u64,
    pub tags: &'p str,
}

pub struct Moebooru<'m> {
    pub root: &'m str,
    pub config: PlatformConfig<'m>,
    pub worker: mpsc::Sender<Operation<'m>>,
    pub client: Client,
}

impl<'m> Moebooru<'m> {
    async fn filter(&self, posts: &mut Posts) {
        //use sqlite::Value;
        match self.config.blacklist {
            Some(ref bl) => {
                for &tag in bl {
                    for post in posts.iter_mut() {
                        if post.as_ref().is_some_and(|p| p.tags.contains(tag)) {
                            *post = None;
                        }
                    }
                }
            }
            None => (),
        }
        for post in posts.iter_mut() {
            if post.as_ref().is_some_and(|p| {
                self.config
                    .blacklist
                    .as_ref()
                    .is_some_and(|bl| bl.iter().any(|&tag| p.tags.contains(tag)))
            }) {
                println!("blacklist: {}", post.as_ref().unwrap().id);
                *post = None;
                continue;
            }
            match post {
                Some(_post) => {
                    let (comm_send, comm_recv): (
                        oneshot::Sender<Option<DbEntryOwned>>,
                        oneshot::Receiver<Option<DbEntryOwned>>,
                    ) = oneshot::channel();
                    let op = Operation::Select(Select {
                        platform: self.config.platform,
                        //bindables: Vec::from([(1, Value::Integer(p.id))]),
                        sender: comm_send,
                    });
                    self.worker.send(op).await.unwrap();
                    let result = comm_recv.await.unwrap();
                    match result {
                        Some(r) => {
                            if r.id == _post.id && r.tags != _post.tags {
                                _post.is_duplicate = true;
                                _post.db_entry = Some(r);
                            }
                        }
                        None => (),
                    }
                }
                None => (),
            }
        }
    }

    async fn map_tag_types(&self, tag_map: Map<String, serde_json::Value>) -> TagMap {
        let mut map: TagMap = TagMap::new();
        for (_tag, _type) in tag_map {
            // yandere stores tags in keys and types in values
            // like: {"asuma_toki": "character"}
            match _type.as_str().unwrap() {
                GENERAL => map.general.push(_tag),
                CHARACTER => map.character.push(_tag),
                COPYRIGHT => map.copyright.push(_tag),
                ARTIST => map.artist.push(_tag),
                METADATA => map.metadata.push(_tag),
                CIRCLE => map.circle.push(_tag),
                FAULTS => map.faults.push(_tag),
                _ => panic!("unexpected tag type: {}", _type.as_str().unwrap()),
            }
        }
        return map;
    }

    async fn to_kw_map<'post>(
        &'post self,
        post: &'post MoePost,
        tag_map: &'post TagMap,
    ) -> KeywordMap {
        return KeywordMap {
            platform: self.config.platform,
            id: post.id,
            tags: post.tags.as_str(),
            source: post.source.as_str(),
            md5: post.md5.as_str(),
            file_size: post.file_size,
            file_ext: post.file_ext.as_str(),
            rating: post.rating.as_str(),
            path: "",
            general: tag_map
                .general
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            character: tag_map
                .character
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            copyright: tag_map
                .copyright
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            artist: tag_map
                .artist
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            metadata: tag_map
                .metadata
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            circle: tag_map
                .circle
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            faults: tag_map
                .faults
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
        };
    }

    async fn handle_duplicate<'dup>(
        &self,
        db_entry: DbEntry,
        duplicate_entry: &DbEntryOwned,
        kw_map: KeywordMap<'dup>,
    ) {
        if self.config.to_cloud {
            rclone::moveto(
                format!(
                    "{cloud}:{src}",
                    cloud = self.config.cloud.unwrap(),
                    src = duplicate_entry.path.as_str(),
                    //filename = duplicate_entry.filename.as_str()
                ),
                format!(
                    "{cloud}:{dest}",
                    cloud = self.config.cloud.unwrap(),
                    dest = kw_map.path,
                    //filename = db_entry.filename
                ),
                || async {},
            )
            .await;
            if self.config.compress {
                if !duplicate_entry.compress_path.is_empty() {
                    let dest: String = match self.config.compress_subdir {
                        Some(ref sb) => [
                            self.config.compress_base.as_ref().unwrap().as_str(),
                            sb.as_str(),
                            self.config.compress_filename.unwrap(),
                        ]
                        .join("/"),
                        None => [
                            self.config.compress_base.as_ref().unwrap().as_str(),
                            self.config.compress_filename.unwrap(),
                        ]
                        .join("/"),
                    };
                    let dest_fmt = kw_map.format(dest.as_str()).await;
                    rclone::moveto(
                        format!(
                            "{cloud}:{src}",
                            cloud = self.config.cloud.unwrap(),
                            src = duplicate_entry.compress_path.as_str(),
                        ),
                        format!(
                            "{cloud}:{dest}",
                            cloud = self.config.cloud.unwrap(),
                            dest = dest_fmt.as_str(),
                        ),
                        || async {},
                    )
                    .await;
                }
            }
        } else {
            if self.config.compress {
                if !duplicate_entry.compress_path.is_empty() {
                    let dest: String = match self.config.compress_subdir {
                        Some(ref sb) => [
                            self.config.compress_base.as_ref().unwrap().as_str(),
                            sb.as_str(),
                            self.config.compress_filename.unwrap(),
                        ]
                        .join("/"),
                        None => [
                            self.config.compress_base.as_ref().unwrap().as_str(),
                            self.config.compress_filename.unwrap(),
                        ]
                        .join("/"),
                    };
                    let dest_fmt = kw_map.format(dest.as_str()).await;
                    utils::mvf(
                        duplicate_entry.compress_path.as_str(),
                        dest_fmt.as_str(),
                        || async {},
                    )
                    .await;
                }
            }
            utils::mvf(duplicate_entry.path.as_str(), kw_map.path, || async {}).await;
        }
        self.worker
            .send(Operation::Insert(Insert {
                platform: self.config.platform,
                entry: db_entry,
            }))
            .await
            .unwrap();
    }

    #[allow(unused_assignments)]
    async fn handle_compression(
        &self,
        file: &PathBuf,
        kw_map: &KeywordMap<'_>,
        db_entry: &mut DbEntry,
    ) {
        let mut is_success = false;
        let mut path_vec: Vec<String> = match self.config.compress_subdir {
            Some(ref sb) => Vec::from([
                kw_map
                    .format(self.config.compress_base.as_ref().unwrap().as_str())
                    .await,
                kw_map.format(sb.as_str()).await,
            ]),
            None => Vec::from([kw_map
                .format(self.config.compress_base.as_ref().unwrap().as_str())
                .await]),
        };
        path_vec.push(kw_map.format(self.config.compress_filename.unwrap()).await);
        let (local_send, local_recv): (
            oneshot::Sender<Option<PathBuf>>,
            oneshot::Receiver<Option<PathBuf>>,
        ) = oneshot::channel();
        self.worker
            .send(Operation::Image(ImageRequest {
                src: file.clone(),
                dest: path_vec.clone(),
                size: self.config.compress_size.unwrap(),
                fallback: if self.config.to_cloud {
                    Some(HOME.to_string())
                } else {
                    None
                },
                response_channel: local_send,
            }))
            .await
            .unwrap();
        let resp = local_recv.await.unwrap();
        match resp {
            Some(file) => {
                if self.config.to_cloud {
                    let _dest_path = path_vec.get(0..path_vec.len() - 1).unwrap().join("/");
                    is_success = rclone::upload(
                        file.to_str().unwrap(),
                        _dest_path.as_str(),
                        self.config.delete,
                        || async {},
                    )
                    .await;
                } else {
                    is_success = true;
                }
            }
            None => is_success = false,
        }
        if is_success {
            db_entry.compress_path = Some(path_vec.join("/"));
        }
    }

    #[allow(unused_assignments)]
    async fn post_task<'post>(&'post self, post: MoePost, tag_map: &'post TagMap) {
        let mut is_success = false;
        let mut kw_map = self.to_kw_map(&post, tag_map).await;
        let base_dir: String = kw_map.format(self.config.base_dir.as_str()).await;
        let output_dir: Option<String> = match self.config.output_dir {
            Some(o) => Some(kw_map.format(o).await),
            None => None,
        };
        let filename: String = kw_map.format(self.config.filename).await;
        let full_path_vec: Vec<&str> = match output_dir {
            Some(ref o) => Vec::from([base_dir.as_str(), o.as_str(), filename.as_str()]),
            None => Vec::from([base_dir.as_str(), filename.as_str()]),
        };
        let full_path: String = full_path_vec.join("/");
        kw_map.path = full_path.as_str();
        let mut db_entry = DbEntry {
            id: post.id,
            md5: kw_map.md5.into(),
            source: post.source.clone(),
            tags: post.tags.clone(),
            path: kw_map.path.to_string(),
            compress_path: None,
        };
        if post.is_duplicate {
            match post.db_entry {
                Some(ref duplicate_entry) => {
                    return self.handle_duplicate(
                        db_entry,
                        duplicate_entry,
                        kw_map,
                    ).await;
                },
                None => panic!("unexpected event: is_downloaded is true but encountered None for db_entry\npost: {:?}", post),
            }
        } else {
            let downloaded = download(
                &self.client,
                post.file_url.as_str(),
                full_path_vec,
                Some(HOME.as_str()),
                self.config.timeout,
                self.config.retries,
                self.config.retry_sleep,
            )
            .await;
            match downloaded {
                Some(file) => {
                    if self.config.compress {
                        self.handle_compression(&file, &kw_map, &mut db_entry).await;
                    }
                    if self.config.to_cloud {
                        is_success = rclone::upload(
                            file.to_str().unwrap(),
                            format!("{}:{}", self.config.cloud.unwrap(), full_path.as_str())
                                .as_str(),
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
        }
        if is_success {
            self.worker
                .send(Operation::Insert(Insert {
                    platform: self.config.platform,
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
            let mut posts: Posts = serde_json::from_value(response[POSTS].take()).unwrap();
            let posts_len = posts.len();
            if posts_len == 0 {
                return None;
            }
            if self.config.skip {
                self.filter(&mut posts).await;
            }
            let tag_map: TagMap = {
                let _map: Map<String, serde_json::Value> =
                    serde_json::from_value(response[TAGS].take()).unwrap();
                self.map_tag_types(_map).await
            };
            return Some((posts, tag_map));
        }
    }

    pub async fn main(&self) {
        let mut params = Params {
            api_version: 2,
            include_tags: 1,
            limit: 100,
            page: 0,
            tags: "",
        };
        for &tag in self.config.tags.iter() {
            params.page = 0;
            println!("{}: {}", self.config.platform, tag);
            params.tags = tag;
            while let Some((posts, tag_map)) = {
                params.page += 1;
                self.tag_task(&params, &self.client /*, &cmp*/).await
            } {
                for post in posts {
                    match post {
                        Some(p) if p.status.as_str() != "deleted" => {
                            sleep(self.config.sleep).await;
                            self.post_task(p, &tag_map).await;
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}
