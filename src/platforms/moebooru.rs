use super::base::{self, PlatformConfig};
use crate::consts::*;
use crate::worker::{ImageRequest, /*ImageResponse, */ Insert, Operation};
use crate::{downloader, json_utils, rclone, string, utils};
use reqwest::Client;
use serde::Serialize;
use serde_json::{Map, Value};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

const TAGS: &str = "tags";
const PAGE: &str = "page";
const POSTS: &str = "posts";

pub struct Moebooru<'m> {
    root: &'m str,
    config: PlatformConfig<'m>,
    worker: mpsc::Sender<Operation>,
    client: Client,
}

impl<'m> Moebooru<'m> {
    pub fn new(
        config: PlatformConfig<'m>,
        worker: mpsc::Sender<Operation>,
        client: Client,
    ) -> Self {
        Self {
            root: match config.platform {
                YANDERE => YANDERE_ROOT,
                SAKUGABOORU => SAKUGABOORU_ROOT,
                KONACHAN => KONACHAN_ROOT,
                _ => panic!(),
            },
            client: client,
            config: config,
            worker: worker,
        }
    }

    pub async fn post_task<'post>(&self, post: &Map<String, Value>, tag_map: &Map<String, Value>) {
        //println!("post_id: {}", post["id"].as_u64().unwrap());
        let mut kw_map: Map<String, Value> = Map::new();
        match post["tags"] {
            Value::String(ref s) => {
                let tags: Vec<&str> = s.as_str().split_whitespace().collect::<Vec<&str>>();
                for tag in tags {
                    if tag_map.contains_key(tag) {
                        let tag_type: &str = tag_map[tag].as_str().unwrap();
                        if !kw_map.contains_key(tag_type) {
                            kw_map.insert(tag_type.into(), Value::Array(Vec::new()));
                        }
                        kw_map[tag_type].as_array_mut().unwrap().push(tag.into());
                    }
                }
            }
            _ => todo!(),
        }
        if !post.contains_key("file_ext") {
            match post.get("file_url") {
                Some(url) => match url.as_str().unwrap().rsplit_once(".") {
                    Some(split) => {
                        kw_map.insert("file_ext".into(), split.1.into());
                    }
                    None => {
                        eprintln!(
                            "Cannot determine file_ext for: {}",
                            post["id"].as_i64().unwrap()
                        );
                    }
                },
                None => (),
            }
        }
        kw_map.insert("platform".into(), self.config.platform.into());
        let base_dir: String = string::format(
            post,
            Some(&kw_map),
            self.config.base_dir.as_str().trim_end_matches('/'),
            Some(self.config.dname_repl),
        )
        .await;
        let output_dir: Option<String> = match self.config.output_dir {
            Some(o) => Some(
                string::format(
                    post,
                    Some(&kw_map),
                    o.trim_matches('/'),
                    Some(self.config.dname_repl),
                )
                .await,
            ),
            None => None,
        };
        let filename: String = string::format(
            post,
            Some(&kw_map),
            self.config.filename.trim_matches('/'),
            Some(self.config.fname_repl),
        )
        .await;
        kw_map.insert("filename".into(), filename.as_str().into());
        let target_dir: Vec<&str> = match output_dir {
            Some(ref o) => [&base_dir, o]
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>(),
            None => Vec::from([base_dir.as_str()]),
        };
        let target_file: Vec<&str> = match output_dir {
            Some(ref o) => [&base_dir, o, &filename]
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>(),
            None => [&base_dir, &filename]
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>(),
        };
        //println!("target_file: {:?}", target_file);
        let db_entry: Operation = Operation::Insert(Insert {
            unique: self.config.db_unique.to_string(),
            table: self.config.platform.to_string(),
            cols: json_utils::get_keys(&post, &self.config.db_keys, Some(&kw_map)).await,
        });
        if post.contains_key(DUPLICATE)
            && post[DUPLICATE].as_object().is_some_and(|obj| {
                obj.contains_key("filename")
                    && obj["filename"].as_str().is_some_and(|s| s.len() > 0)
            })
        {
            let target_dir: PathBuf = target_dir.iter().collect::<PathBuf>();
            let target_dir_str: &str = target_dir.to_str().unwrap();
            let _src: String = format!(
                "{}/{}",
                target_dir_str,
                post[DUPLICATE]["filename"].as_str().unwrap()
            );
            let _dest: String = format!("{}/{}", target_dir_str, filename.as_str());
            if self.config.to_cloud {
                match self.config.cloud {
                    Some(v) => {
                        rclone::moveto(
                            format!("{}:{}", v, _src),
                            format!("{}:{}", v, _dest),
                            || async {},
                        )
                        .await;
                        //on_success().await;
                        self.worker.send(db_entry).await.unwrap();
                    }
                    None => (),
                }
            } else {
                utils::mvf(_src.as_str(), _dest.as_str(), || async {}).await;
                //on_success().await;
                self.worker.send(db_entry).await.unwrap();
            }
        } else {
            let downloaded: PathBuf = match downloader::main(
                &self.client,
                post["file_url"].as_str().unwrap(),
                target_file.clone(),
                if self.config.to_cloud {
                    Some(self.config.home_dir.as_str())
                } else {
                    None
                },
                self.config.timeout,
                self.config.retries,
                self.config.retry_sleep,
            )
            .await
            {
                Some(r) => r,
                None => return,
            };
            if self.config.compress {
                let compress_dest: Vec<String> = string::format_multiple(
                    &post,
                    Some(&kw_map),
                    //None,
                    match self.config.compress_subdir {
                        Some(ref d) => Vec::from([
                            self.config
                                .compress_base
                                .as_ref()
                                .unwrap()
                                .as_str()
                                .trim_end_matches('/'),
                            d.as_str().trim_matches('/'),
                        ]),
                        None => Vec::from([self
                            .config
                            .compress_base
                            .as_ref()
                            .unwrap()
                            .as_str()
                            .trim_end_matches('/')]),
                    },
                    Some(self.config.dname_repl),
                )
                .await;
                let filename: String = string::format(
                    &post,
                    Some(&kw_map),
                    self.config.compress_filename.unwrap().trim_matches('/'),
                    Some(self.config.fname_repl),
                )
                .await;
                // compress_dest.push(filename.to_string());
                let mut compress_dest_full = compress_dest.clone();
                compress_dest_full.push(filename.to_string());
                /*let mut compress_dest_str = compress_dest
                    .iter()
                    .map(|x| x.as_str())
                    .collect::<Vec<&str>>();
                compress_dest_str.push(filename.as_str());*/
                let (send, recv) = oneshot::channel();
                let image_request = ImageRequest {
                    src: downloaded.clone(),
                    dest: compress_dest_full,
                    size: self.config.compress_size.unwrap(),
                    fallback: if self.config.to_cloud {
                        Some(self.config.home_dir.clone())
                    } else {
                        None
                    },
                    response_channel: send,
                };
                self.worker
                    .send(Operation::Image(image_request))
                    .await
                    .unwrap();
                match recv.await.unwrap() {
                    Some(response) => {
                        if self.config.to_cloud {
                            rclone::upload(
                                response.file.to_str().unwrap(),
                                format!(
                                    "{}:{}",
                                    self.config.cloud.unwrap(),
                                    compress_dest.join("/")
                                )
                                .as_str(),
                                self.config.delete,
                                || async {},
                            )
                            .await;
                        }
                    }
                    None => (),
                }
                if self.config.to_cloud {
                    let dest: String = target_dir.join("/");
                    let dest_fmt: String =
                        format!("{}:{}", self.config.cloud.unwrap(), dest.as_str());
                    rclone::upload(
                        downloaded.to_str().unwrap(),
                        dest_fmt.as_str(),
                        self.config.delete,
                        || async { self.worker.send(db_entry).await.unwrap() },
                    )
                    .await;
                } else {
                    self.worker.send(db_entry).await.unwrap();
                }
            }
        }
    }

    pub async fn tag_task<T: Serialize + ?Sized>(
        &self,
        params: &T,
        client: &Client,
        cmp: &Vec<&str>,
    ) -> Option<Value> {
        let retval: &str = "filename";
        loop {
            let mut resp: Value = {
                loop {
                    match client.get(self.root).query(params).send().await {
                        Ok(r) => match r.json::<Value>().await {
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
            //let mut resp: Value = base::get_json(client.get(self.root).query(params)).await;
            if resp[POSTS].as_array().is_some_and(|a| a.len() == 0) {
                return None;
            }
            if self.config.skip {
                match base::filter(
                    self.worker.clone(),
                    self.config.platform,
                    resp[POSTS].take(),
                    self.config.db_unique,
                    cmp,
                    Some(retval),
                    match self.config.blacklist {
                        Some(ref v) => Some(v),
                        None => None,
                    },
                )
                .await
                {
                    Some(posts) => {
                        resp[POSTS] = posts;
                        return Some(resp);
                    }
                    None => return None,
                }
            }
        }
    }

    pub async fn main(&self) {
        let mut params: Map<String, Value> = Map::new();
        params.insert("api_version".to_string(), 2.into());
        params.insert("include_tags".to_string(), 1.into());
        params.insert("limit".to_string(), 100.into());

        let cmp: Vec<&str> = Vec::from([TAGS]);
        'tag_iter: for &tag in self.config.tags.iter() {
            let mut page: u64 = 0;
            println!("{}: {}", self.config.platform, tag);
            if params.contains_key(TAGS) {
                params.remove(TAGS);
            }
            params.insert(TAGS.to_string(), tag.into());
            while let Some(mut resp) = {
                page += 1;
                if params.contains_key(PAGE) {
                    params.remove(PAGE);
                }
                params.insert(PAGE.to_string(), page.into());
                self.tag_task(&params, &self.client, &cmp).await
            } {
                let tag_map: Value = resp[TAGS].take();
                match resp[POSTS].as_array_mut() {
                    Some(a) => {
                        for post in a {
                            match post {
                                Value::Object(o) => {
                                    if !o.get("status").is_some_and(|st| {
                                        st.as_str().is_some_and(|s| s == "deleted")
                                    }) {
                                        self.post_task(o, tag_map.as_object().unwrap()).await;
                                        sleep(self.config.sleep).await;
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                    None => continue 'tag_iter,
                }
            }
        }
    }
}
