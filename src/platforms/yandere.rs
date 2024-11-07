use crate::platforms::base::{self, PlatformConfig};
use crate::{db, downloader, json_utils, rclone, string, utils};
//use json::{self, object, Value};
use reqwest::{Client, RequestBuilder};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;
use tokio::time::{sleep, Duration};

const ROOT: &str = "https://yande.re/post.json";
const PLATFORM: &str = "yandere";

async fn post_task<'post>(
    post: &mut Map<String, Value>,
    tags_metadata: &Map<String, Value>,
    config: &PlatformConfig<'post>,
) {
    let mut tag_map: Map<String, Value> = Map::new();
    if post["tags"].is_string() {
        let _tags: Vec<&str> = post["tags"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<&str>>();
        for _tag in _tags {
            if tags_metadata.contains_key(_tag) {
                let tag_type: &str = tags_metadata[_tag].as_str().unwrap();
                match tag_map[tag_type] {
                    Value::Null => tag_map[tag_type] = Value::Array(Vec::from([_tag.into()])),
                    Value::Array(ref mut a) => a.push(_tag.into()),
                    _ => panic!(),
                };
            }
        }
    }
    let extra: Vec<(&str, &str)> = Vec::from([("platform", PLATFORM)]);
    let dir_restrict: Vec<char> = Vec::from([':', '!', '?', '"', '*', '\'']);
    let fname_restrict: Vec<char> = Vec::from([':', '!', '?', '"', '*', '\'', '/']);

    let base_dir: String = string::format(
        &post,
        Some(&tag_map),
        Some(&extra),
        config.base_dir.as_str(),
        Some(&dir_restrict),
    )
    .await;
    let output_dir: Option<String> = match config.output_dir {
        Some(o) => {
            Some(string::format(&post, Some(&tag_map), Some(&extra), o, Some(&dir_restrict)).await)
        }
        None => None,
    };
    let filename: String = string::format(
        &post,
        Some(&tag_map),
        Some(&extra),
        config.filename,
        Some(&fname_restrict),
    )
    .await;
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

    let db_entry: Map<String, Value> = json_utils::get_keys(
        &post,
        &config.db_keys,
        Some({
            let mut _extra: Map<String, Value> = Map::new();
            _extra.insert("filename".into(), filename.as_str().into());
            _extra
        }),
    )
    .await;
    let on_success = || async {
        db::add_to_db(
            &config.database,
            config.db_unique,
            &config.platform,
            db_entry,
        )
        .await;
    };
    if post.contains_key("_duplicate_") {
        let target_dir: PathBuf = target_dir.iter().collect::<PathBuf>();
        let target_dir_str: &str = target_dir.to_str().unwrap();
        let _src: String = format!(
            "{}/{}",
            target_dir_str,
            post["_duplicate_"]["filename"].as_str().unwrap()
        );
        let _dest: String = format!("{}/{}", target_dir_str, filename.as_str());
        if config.to_cloud {
            match config.cloud {
                Some(v) => {
                    rclone::moveto(
                        format!("{}:{}", v, _src),
                        format!("{}:{}", v, _dest),
                        on_success,
                    )
                    .await;
                }
                None => {
                    println!("to_cloud is true but cloud value is empty!");
                    exit(1)
                }
            }
        } else {
            utils::mvf(_src.as_str(), _dest.as_str(), on_success).await;
        }
    } else {
        let downloaded: PathBuf = downloader::main(
            post["file_url"].as_str().unwrap(),
            target_file.clone(),
            if config.to_cloud {
                Some(config.home_dir.as_str())
            } else {
                None
            },
            config.timeout,
            config.retry,
        )
        .await;
        if config.compress {
            let compress_dest: Vec<String> = string::format_multiple(
                &post,
                Some(&tag_map),
                Some(&extra),
                match config.compress_subdir {
                    Some(ref d) => Vec::from([
                        string::strip_last(config.compress_base.as_ref().unwrap().as_str(), '/'),
                        string::strip(d.as_str(), '/'),
                    ]),
                    None => Vec::from([string::strip_last(
                        config.compress_base.as_ref().unwrap().as_str(),
                        '/',
                    )]),
                },
                Some(&dir_restrict),
            )
            .await;
            let filename: String = string::format(
                &post,
                Some(&tag_map),
                Some(&extra),
                string::strip(config.compress_filename.unwrap(), '/'),
                Some(&fname_restrict),
            )
            .await;
            let mut compress_dest_str = compress_dest
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>();
            compress_dest_str.push(filename.as_str());
            match utils::image_resize(
                &downloaded,
                compress_dest_str,
                config.compress_size.as_ref().unwrap(),
                if config.to_cloud {
                    Some(config.home_dir.as_str())
                } else {
                    None
                },
                || (),
            )
            .await
            {
                Some(rp) => {
                    if config.to_cloud {
                        rclone::upload(
                            rp,
                            format!("{}:{}", config.cloud.unwrap(), compress_dest.join("/"))
                                .as_str(),
                            config.delete,
                            || async { () },
                        )
                        .await;
                    }
                }
                None => (),
            }
            if config.to_cloud {
                let dest: String = target_dir.join("/");
                let dest_fmt: String = format!("{}:{}", config.cloud.unwrap(), dest.as_str());
                rclone::upload(downloaded, dest_fmt.as_str(), config.delete, on_success).await;
            } else {
                on_success().await;
            }
        }
    }
}

async fn tag_task<'task>(
    _params: &HashMap<&str, &'task str>,
    client: &Client,
    config: &PlatformConfig<'task>,
    cmp: &Vec<&str>,
) {
    let retval: &str = "filename";
    let mut page: usize = 1;
    let req_build: RequestBuilder = client.get(ROOT).query(&_params);
    loop {
        let mut resp: Value = base::get_json(match req_build.try_clone() {
            Some(c) => c.query(&[("page", page)]),
            _ => panic!(),
        })
        .await;
        if resp["posts"].as_array().is_some_and(|ref a| a.len() == 0) {
            break;
        }
        if config.skip {
            match base::filter(
                PathBuf::from(&config.database),
                &config.platform,
                resp["posts"].take(),
                config.db_unique,
                cmp,
                Some(&retval),
            )
            .await
            {
                Some(v) => {
                    resp["posts"] = v;
                }
                None => return,
            }
        }
        let tags_metadata: Value = resp["tags"].take();
        //println!("{:?}", resp["posts"]);
        match resp["posts"] {
            Value::Array(ref mut a) => {
                for post in a {
                    post_task(
                        post.as_object_mut().unwrap(),
                        tags_metadata.as_object().unwrap(),
                        &config,
                    )
                    .await;
                    sleep(Duration::new(1, 0)).await;
                }
            }
            _ => panic!("invalid json data type encountered: {:?}", resp["posts"]),
        }
        page += 1;
    }
}

pub async fn main<'yandere>(conf: PlatformConfig<'yandere>) {
    let mut _params: HashMap<&str, &str> = HashMap::new();
    _params.insert("api_version", "2");
    _params.insert("include_tags", "1");
    _params.insert("limit", "100");
    let client: Client = Client::new();
    let cmp: Vec<&str> = Vec::from(["id", "tags"]);
    for tag in conf.tags.iter() {
        let tag: &str = *tag;
        if _params.contains_key("tags") {
            _params.remove("tags");
        }
        _params.insert("tags", tag);
        println!("yandere tag: {tag}");
        tag_task(&mut _params, &client, &conf, &cmp).await;
    }
}
