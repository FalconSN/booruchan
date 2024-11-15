use super::{Gelbooru, Moebooru};
use crate::{consts::*, json_utils, worker::Operation, HOME};
use json::Map;
use reqwest::Client;
use serde_json as json;
use std::{env, io::ErrorKind, path::PathBuf, process::exit};
use tokio::{sync::mpsc, time::Duration};

pub enum Platform {
    Moebooru,
    Gelbooru,
}

impl Platform {
    pub async fn init<'p>(
        config: PlatformConfig<'p>,
        worker: mpsc::Sender<Operation<'p>>,
        client: Client,
    ) -> () {
        let (root, platform) = match config.platform {
            YANDERE => (YANDERE_ROOT, Platform::Moebooru),
            SAKUGABOORU => (SAKUGABOORU_ROOT, Platform::Moebooru),
            KONACHAN => (KONACHAN_ROOT, Platform::Moebooru),
            GELBOORU => (GELBOORU_ROOT, Platform::Gelbooru),
            _ => panic!(),
        };
        match platform {
            Platform::Moebooru => {
                return Moebooru {
                    root: root,
                    config: config,
                    worker: worker,
                    client: client,
                }
                .main()
                .await
            }
            Platform::Gelbooru => {
                return Gelbooru {
                    root: root,
                    config: config,
                    worker: worker,
                    client: client,
                }
                .main()
                .await
            }
        }
    }
}

pub struct PlatformConfig<'a> {
    pub platform: &'a str,
    pub to_cloud: bool,
    pub delete: bool,
    pub cloud: Option<&'a str>,
    pub database: PathBuf,
    /*pub db_unique: &'a str,
    pub db_keys: Vec<&'a str>,*/
    pub base_dir: String,
    pub output_dir: Option<&'a str>,
    pub filename: &'a str,
    pub compress: bool,
    pub compress_db: Option<PathBuf>,
    pub compress_base: Option<String>,
    pub compress_subdir: Option<String>,
    pub compress_filename: Option<&'a str>,
    pub compress_size: Option<(u32, u32)>,
    pub skip: bool,
    pub sleep: Duration,
    pub retries: i64,
    pub retry_sleep: Duration,
    pub timeout: Duration,
    pub fname_repl: &'a json::Value,
    pub dname_repl: &'a json::Value,
    pub tags: Vec<&'a str>,
    pub blacklist: Option<Vec<&'a str>>,
}

pub fn parse_config<'c>(
    conf: &'c Map<String, json::Value>,
    platform: &'c str,
) -> Result<PlatformConfig<'c>, ()> {
    let to_cloud: bool = match json_utils::get_value(conf, "to_cloud", platform) {
        Some(v) => v.as_bool().unwrap(),
        None => false,
    };
    let delete: bool = match json_utils::get_value(conf, "delete", platform) {
        Some(v) => v.as_bool().unwrap(),
        None => false,
    };
    let cloud: Option<&str> = {
        if to_cloud {
            match json_utils::get_value(conf, "cloud", platform) {
                Some(v) => Some(v.as_str().unwrap()),
                None => panic!("cloud value can't be empty when to_cloud is true!"),
            }
        } else {
            None
        }
    };
    let database: PathBuf = match json_utils::get_value(conf, "database", platform) {
        Some(v) => PathBuf::from(v.as_str().unwrap()),
        None => PathBuf::from(HOME.as_str())
            .join(".archives")
            .join("booruchan.db"),
    };

    /*
    let db_unique: &str = match json_utils::get_value(conf, "db_unique", platform) {
        Some(v) => v.as_str().unwrap(),
        None => "id",
    };

    let db_keys: Vec<&str> = match json_utils::get_value(conf, "db_keys", platform) {
        Some(val) => {
            let mut _temp: Vec<&str> = Vec::new();
            match val {
                json::Value::Array(ref a) => {
                    a.iter().for_each(|v| match v {
                        json::Value::String(s) => _temp.push(s.as_str()),
                        _ => {
                            panic!("unknown data type for db_keys: {:?}", v);
                        }
                    });
                }
                json::Value::String(ref s) => {
                    for val in s.as_str().split_whitespace() {
                        _temp.push(val);
                    }
                }
                _ => todo!(),
            };
            _temp
        }
        None => Vec::from(["id", "md5", "source", "tags", "filename"]),
    };
    */
    let base_dir: String = match json_utils::get_value(conf, "base_dir", platform) {
        Some(val) => String::from(val.as_str().unwrap().trim_end_matches('/')),
        None => match env::current_dir() {
            Ok(v) => String::from(v.to_str().unwrap()),
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    println!("are you in a non-existent directory?");
                    exit(1)
                }
                _ => panic!("{:?}", e),
            },
        },
    };
    let output_dir: Option<&str> = match json_utils::get_value(conf, "output_dir", platform) {
        Some(v) => Some(v.as_str().unwrap().trim_matches('/')),
        None => None,
    };
    let filename: &str = match json_utils::get_value(conf, "filename", platform) {
        Some(v) => v.as_str().unwrap().trim_matches('/'),
        None => "{id}.{file_ext}",
    };
    let compress: bool = match json_utils::get_value(conf, "compress", platform) {
        Some(v) => v.as_bool().unwrap(),
        None => false,
    };
    let compress_db: Option<PathBuf> = {
        if compress {
            match json_utils::get_value(&conf, "compress_db", platform) {
                Some(v) => Some(PathBuf::from(v.as_str().unwrap())),
                None => Some(PathBuf::from(
                    [HOME.as_str(), ".archives", "compress.db"]
                        .iter()
                        .collect::<PathBuf>(),
                )),
            }
        } else {
            None
        }
    };
    let mut compress_base: Option<String> = {
        if compress {
            match json_utils::get_value(conf, "compress_base", platform) {
                Some(v) => Some(String::from(v.as_str().unwrap())),
                None => Some(base_dir.clone()),
            }
        } else {
            None
        }
    };
    let compress_subdir: Option<String> = {
        if compress {
            match json_utils::get_value(conf, "compress_subdir", platform) {
                Some(v) => Some(v.to_string()),
                None => match output_dir {
                    Some(d) => Some(d.to_owned() + "_compressed"),
                    None => match compress_base {
                        Some(b) => {
                            compress_base = Some(b + "_compressed");
                            None
                        }
                        None => todo!(),
                    },
                },
            }
        } else {
            None
        }
    };
    let compress_filename: Option<&str> = {
        if compress {
            match json_utils::get_value(conf, "compress_filename", platform) {
                Some(v) => Some(v.as_str().unwrap()),
                None => Some(filename),
            }
        } else {
            None
        }
    };
    let compress_size: Option<(u32, u32)> = if compress {
        match json_utils::get_value(conf, "compress_size", platform) {
            Some(v) => Some((v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32)),
            None => Some((5000, 5000)),
        }
    } else {
        None
    };
    let skip: bool = match json_utils::get_value(conf, "skip", platform) {
        Some(v) => v.as_bool().unwrap(),
        None => false,
    };
    let sleep: Duration = match json_utils::get_value(conf, "sleep", platform) {
        Some(v) => Duration::from_secs_f64(v.as_f64().unwrap()),
        None => Duration::from_secs(1),
    };
    let retries: i64 = match json_utils::get_value(conf, "retries", platform) {
        Some(v) => v.as_i64().unwrap(),
        None => -1,
    };
    let retry_sleep: Duration = match json_utils::get_value(conf, "retry-sleep", platform) {
        Some(v) => Duration::from_secs_f64(v.as_f64().unwrap()),
        None => Duration::from_secs(3),
    };
    let timeout: Duration = match json_utils::get_value(conf, "timeout", platform) {
        Some(t) => match t {
            json::Value::Number(ref n) => {
                if n.is_f64() {
                    Duration::from_secs_f64(n.as_f64().unwrap())
                } else {
                    Duration::from_secs(n.as_u64().unwrap())
                }
            }
            _ => panic!("unexpected data type for timeout, expected f64 or i64"),
        },
        None => Duration::from_secs(30),
    };
    let filename_replace: &json::Value =
        match json_utils::get_value(conf, "filename-replace", platform) {
            Some(v) => v,
            None => {
                let temp = Box::leak(Box::new(json::Value::Array(Vec::new())));
                let as_arr = temp.as_array_mut().unwrap();
                [":", "!", "?", "*", "\"", "'", "/"]
                    .iter_mut()
                    .for_each(|c| {
                        if c.len() > 1 {
                            panic!("filename_replace: expected a character, found a string = {c}");
                        }
                        as_arr.push(c.to_string().into());
                    });
                temp
            }
        };
    let dirname_replace: &json::Value =
        match json_utils::get_value(conf, "dirname-replace", platform) {
            Some(v) => v,
            None => {
                let temp = Box::leak(Box::new(json::Value::Array(Vec::new())));
                let as_arr = temp.as_array_mut().unwrap();
                [":", "!", "?", "*", "\"", "'"].iter_mut().for_each(|c| {
                    if c.len() > 1 {
                        panic!("dirname_replace: expected a character, found a string = {c}");
                    }
                    as_arr.push(c.to_string().into());
                });
                temp
            }
        };
    let tags: Vec<&str> = match json_utils::get_value(conf, "tags", platform) {
        Some(v) => {
            let mut _tags: Vec<&str> = Vec::new();
            for item in v.as_array().unwrap() {
                _tags.push(item.as_str().unwrap());
            }
            _tags
        }
        None => Vec::new(),
    };
    let blacklist: Option<Vec<&str>> = match json_utils::get_value(conf, "blacklist", platform) {
        Some(v) => Some(
            v.as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<&str>>(),
        ),
        None => None,
    };

    let config: PlatformConfig = PlatformConfig {
        platform: platform,
        to_cloud: to_cloud,
        delete: delete,
        cloud: cloud,
        database: database,
        /*db_unique: db_unique,
        db_keys: db_keys,*/
        base_dir: base_dir,
        output_dir: output_dir,
        filename: filename,
        compress: compress,
        compress_db: compress_db,
        compress_base: compress_base,
        compress_subdir: compress_subdir,
        compress_filename: compress_filename,
        compress_size: compress_size,
        skip: skip,
        sleep: sleep,
        retries: retries,
        retry_sleep: retry_sleep,
        timeout: timeout,
        fname_repl: filename_replace,
        dname_repl: dirname_replace,
        tags: tags,
        blacklist: blacklist,
    };
    return Ok(config);
}
