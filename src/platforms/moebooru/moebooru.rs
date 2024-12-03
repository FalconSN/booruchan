// std imports
use std::path::PathBuf;

// crate imports
use reqwest::Client;
use serde::Serialize;
use serde_json::Map;
use tokio::{
    sync::{mpsc, oneshot},
    time::{sleep, Duration},
};

// local imports
use super::{Params, Post, Posts, Status};
use crate::{
    config::{Compress, PlatformConfig},
    fmt::Keywords,
    platforms::base::TagMap,
    rclone,
    statics::HOME,
    utils,
    worker::{DbEntry, ImageRequest, Insert, Operation, Select},
    Downloader,
};

const TAGS: &str = "tags";
//const POSTS: &str = "posts";
const GENERAL: &str = "general";
const CHARACTER: &str = "character";
const ARTIST: &str = "artist";
const COPYRIGHT: &str = "copyright";
const METADATA: &str = "metadata";
const CIRCLE: &str = "circle";
const FAULTS: &str = "faults";
const STYLE: &str = "style";

struct Timer {
    retry_sleep: Duration,
    timeout: Duration,
    sleep: Duration,
}

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
                STYLE => map.style.push(_tag),
                _ => panic!("unexpected tag type: {}", _type.as_str().unwrap()),
            }
        }
        return map;
    }

    async fn to_keywords<'post>(
        &'post self,
        post: &'post Post,
        tag_map: &'post TagMap,
    ) -> Keywords {
        return Keywords {
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

    async fn handle_duplicate<'dup>(&self, db_entry: DbEntry, duplicate_entry: &DbEntry) -> bool {
        let mut success: bool = true;
        if db_entry.path != duplicate_entry.path {
            if self.config.to_cloud {
                let cloud = self.config.cloud.as_ref().unwrap().as_str();
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
        match db_entry.compress_path {
            Some(ref cp) => match duplicate_entry.compress_path {
                Some(ref dcp) => {
                    let src = dcp.as_str();
                    let dest = cp.as_str();
                    if self.config.to_cloud {
                        let cloud = self.config.cloud.as_ref().unwrap().as_str();
                        rclone::moveto(
                            format!("{cloud}:{src}",),
                            format!("{cloud}:{dest}"),
                            || async {},
                        )
                        .await;
                    } else {
                        utils::mvf(src, dest, || async {}).await;
                    }
                }
                None => success = false,
            },
            None => (),
        }
        if success {
            self.worker
                .send(Operation::Insert(Insert {
                    platform: self.platform,
                    entry: db_entry,
                }))
                .await
                .unwrap();
        }
        return success;
    }

    async fn handle_compression(
        &self,
        file: &PathBuf,
        compress: &Compress,
        kw_map: &Keywords<'_>,
        db_entry: &mut DbEntry,
    ) {
        let path: Vec<String> = match compress.subdir {
            Some(ref subdir) => [
                compress.base_dir.as_str(),
                subdir.as_str(),
                compress.filename.as_str(),
            ]
            .iter()
            .map(|&s| kw_map.format(s))
            .collect::<Vec<String>>(),
            None => [compress.base_dir.as_str(), compress.filename.as_str()]
                .iter()
                .map(|&s| kw_map.format(s))
                .collect::<Vec<String>>(),
        };
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
                    if rclone::upload(
                        file.to_str().unwrap(),
                        _dest_path.as_str(),
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
    async fn post_task<'post>(&'post self, post: Post, tag_map: &'post TagMap) {
        let mut is_success = false;
        let kw = self.to_keywords(&post, tag_map).await;
        let base_dir: String = kw.format(self.config.base_dir.as_str());
        let output_dir: Option<String> = match self.config.subdir {
            Some(ref o) => Some(kw.format(o.as_str())),
            None => None,
        };
        let filename: String = kw.format(self.config.filename.as_str());
        let full_path_vec: Vec<&str> = match output_dir {
            Some(ref o) => Vec::from([base_dir.as_str(), o.as_str(), filename.as_str()]),
            None => Vec::from([base_dir.as_str(), filename.as_str()]),
        };
        let full_path: String = full_path_vec.join("/");
        let mut db_entry = DbEntry {
            id: post.id,
            md5: kw.md5.to_string(),
            source: post.source.clone(),
            tags: post.tags.clone(),
            path: full_path.clone(),
            compress_path: None,
        };
        if post.is_duplicate {
            match post.duplicate_entry {
                Some(ref duplicate_entry) => {
                    if db_entry != *duplicate_entry
                    && self.handle_duplicate(db_entry.clone(), duplicate_entry).await {
                        return;
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
                        self.handle_compression(&file, compress, &kw, &mut db_entry)
                            .await
                    }
                    None => (),
                }
                if self.config.to_cloud {
                    is_success = rclone::upload(
                        file.to_str().unwrap(),
                        format!(
                            "{}:{}",
                            self.config.cloud.as_ref().unwrap(),
                            full_path.as_str()
                        )
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
            let tag_map: TagMap = {
                let _map: Map<String, serde_json::Value> =
                    serde_json::from_value(response[TAGS].take()).unwrap();
                self.map_tag_types(_map).await
            };
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
