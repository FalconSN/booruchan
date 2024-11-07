use crate::consts::DUPLICATE;
use crate::worker::{Operation, Select};
use json::Map;
use serde_json as json;
use sqlite::Value;
use std::{env, io::ErrorKind, path::PathBuf, process::exit};
use tokio::{
    sync::{mpsc, oneshot},
    time::Duration,
};

use crate::json_utils;

pub struct PlatformConfig<'a> {
    pub platform: &'a str,
    pub home_dir: String,
    pub to_cloud: bool,
    pub delete: bool,
    pub cloud: Option<&'a str>,
    pub database: PathBuf,
    pub db_unique: &'a str,
    pub db_keys: Vec<&'a str>,
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
    /*pub timeout: (
        Duration, /* total deadline */
        Duration, /* read timeout */
        Duration, /* connect timeout */
    ),*/
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
    let home: String = match env::var("HOME") {
        Ok(v) => v.to_string(),
        Err(_) => {
            println!("HOME variable is not set, cannot find config!");
            exit(1);
        }
    };
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
        None => PathBuf::from(&home).join(".archives").join("booruchan.db"),
    };
    //let db_unique: &str = "id";

    let db_unique: &str = match json_utils::get_value(conf, "db_unique", platform) {
        Some(v) => v.as_str().unwrap(),
        None => "id",
    };

    //let db_keys: Vec<&str> = Vec::from(["id", "md5", "source", "tags", "filename"]);
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
    let base_dir: String = match json_utils::get_value(conf, "base_dir", platform) {
        Some(val) => String::from(val.as_str().unwrap()),
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
        Some(v) => Some(v.as_str().unwrap()),
        None => None,
    };
    let filename: &str = match json_utils::get_value(conf, "filename", platform) {
        Some(v) => v.as_str().unwrap(),
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
                    [home.as_str(), ".archives", "compress.db"]
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
    /*let timeout: (
        Duration, // 0. connect timeout
        Duration, // 1. read timeout
        Duration, // 2. total timeout
    ) = match json_utils::get_value(conf, "timeout", platform) {
        Some(v) => match v {
            json::Value::Array(ref a) => (
                Duration::from_secs_f64(a[0].as_f64().unwrap()),
                Duration::from_secs_f64(a[1].as_f64().unwrap()),
                Duration::from_secs_f64(a[2].as_f64().unwrap()),
            ),
            _ => panic!("only array types are accepted for timeout option"),
        },
        None => (
            Duration::from_secs(30),
            Duration::from_secs(10),
            Duration::from_secs(5),
        ),
    };*/
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
        home_dir: home,
        to_cloud: to_cloud,
        delete: delete,
        cloud: cloud,
        database: database,
        db_unique: db_unique,
        db_keys: db_keys,
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

pub async fn filter<'f>(
    worker: mpsc::Sender<Operation>,
    platform: &'f str,
    mut posts: json::Value,
    uniq_key: &'f str,
    db_compare: &Vec<&str>,
    retval: Option<&'f str>,
    blacklist: Option<&Vec<&str>>,
) -> Option<json::Value> {
    if posts.as_array().is_some_and(|a| a.len() == 0) {
        return None;
    }
    let db_cmp_len = db_compare.len();
    //let items = db_compare.join(", ");
    let query = format!(
        "select {items} from {platform} where {uniq_key} = ?",
        items = db_compare.join(", ")
    );
    //let query_str = query.as_str();
    //let (local_sender, mut local_receiver) = mpsc::channel(1);
    'post_iter: for post in posts.as_array_mut().unwrap() {
        match blacklist {
            Some(bl) => {
                for &tag in bl {
                    if post["tags"]
                        .as_str()
                        .unwrap()
                        .split_whitespace()
                        .any(|t| t == tag)
                    {
                        *post = json::Value::Null;
                        continue 'post_iter;
                    }
                }
            }
            None => (),
        }
        let post_as_obj = match *post {
            json::Value::Object(ref mut o) => o,
            json::Value::Null => continue,
            _ => panic!(),
        };
        let post_id: i64 = post_as_obj["id"].as_i64().unwrap();
        let uniq_val = match post_as_obj.get(uniq_key) {
            Some(u) => match *u {
                json::Value::Number(ref n) => Value::Integer(n.as_i64().unwrap().into()),
                json::Value::String(ref s) => Value::String(s.into()),
                _ => panic!("unexpected unique key type: {uniq_key}"),
            },
            None => panic!(),
        };
        let (local_sender, local_receiver) = oneshot::channel();
        worker
            .send(Operation::Select(Select {
                query: query.to_owned(),
                uniq: uniq_key.to_string(),
                bindables: Vec::from([(1, uniq_val.clone())]),
                sender: local_sender,
            }))
            .await
            .unwrap();
        let matches: Option<Map<String, json::Value>> = match local_receiver.await.unwrap() {
            Some(r) => Some(r),
            None => None,
        };
        match matches {
            Some(m) => {
                let match_len = m.len();
                if match_len == db_cmp_len {
                    if db_compare.iter().any(|&key| {
                        post_as_obj
                            .get(key)
                            .is_some_and(|val| m.get(key).is_some_and(|mval| *mval != *val))
                    }) {
                        println!("changed: {post_id}");
                        match retval {
                            Some(r) => {
                                let (local_sender, local_receiver) = oneshot::channel();
                                let query =
                                    format!("select {r} from {platform} where {uniq_key} = ?");
                                let opt = Operation::Select(Select {
                                    query: query,
                                    uniq: uniq_key.to_string(),
                                    bindables: Vec::from([(1, uniq_val)]),
                                    sender: local_sender,
                                });
                                worker.send(opt).await.unwrap();
                                let ret = local_receiver.await.unwrap();
                                match ret {
                                    Some(val) => {
                                        post_as_obj
                                            .insert(DUPLICATE.into(), json::Value::Object(val));
                                    }
                                    None => panic!(),
                                }
                            }
                            None => (),
                        }
                    } else {
                        println!("skip: {post_id}");
                        *post = json::Value::Null;
                    }
                }
            }
            None => (),
        }
        /*
        if match_len == db_cmp_len {
            if db_compare.iter().any(|&key| {
                post_as_obj.get(key).is_some_and(|val| match *val {
                    json::Value::String(ref s) => matches
                        .get(key)
                        .is_some_and(|match_val| match_val.as_str().unwrap() != s.as_str()),
                    json::Value::Number(ref n) => matches.get(key).is_some_and(|match_val| {
                        match_val.as_i64().unwrap() != n.as_i64().unwrap()
                    }),
                    json::Value::Null => matches
                        .get(key)
                        .is_some_and(|match_val| *match_val != json::Value::Null),
                    _ => false,
                })
            }) {
                println!("changed: {}", post_id);
                match retval {
                    Some(r) => {
                        let (local_sender, local_receiver) = oneshot::channel();
                        let query = format!("select {r} from {platform} where {uniq_key} = ?");
                        let opt = Operation::Select(Select {
                            query: query.to_owned(),
                            uniq: uniq_key.to_string(),
                            bindables: Vec::from([(1, uniq_val)]),
                            sender: local_sender,
                        });
                        worker.send(opt).await.unwrap();
                        let ret = local_receiver.await.unwrap();
                        match ret {
                            Some(val) => {
                                post_as_obj.insert(DUPLICATE.into(), json::Value::Object(val));
                            }
                            None => panic!(),
                        }
                    }
                    None => (),
                }
            }
        } else {
            println!("skip: {}", post_id);
            *post = json::Value::Null;
        }*/
    }
    return Some(posts);
}

/*
pub async fn filter<'f, P: AsRef<Path>>(
    sender: Sender<Operation<'f>>,
    platform: &'f str,
    mut posts: json::Value,
    uniq_key: &'f str,
    db_compare: &'f Vec<&'f str>,
    retval: Option<&'f str>,
) -> Option<json::Value> {
    if posts.as_array().is_some_and(|a| a.len() == 0) {
        return None;
    }
    let db_cmp_len: usize = db_compare.iter().filter(|i| **i != uniq_key).count();
    let items: String = db_compare.join(", ");
    let query = format!("select {items} from {platform} where {uniq_key} = :{uniq_key}");
    let query_str = query.as_str();
    for post in posts.as_array_mut().unwrap() {
        let post_as_obj = match post.as_object_mut() {
            Some(o) => o,
            None => panic!("post is not a json object!"),
        };
        let mut matches: Map<String, json::Value> = Map::new();
        let mut statement: Statement = match conn.prepare(query_str) {
            Ok(s) => s,
            Err(e) => match e.code {
                Some(c) => {
                    if c == 1 {
                        return Some(posts);
                    } else {
                        panic!("{e:?}");
                    }
                }
                None => panic!("{e:?}"),
            },
        };
        let uniq_val: Value = match post_as_obj[uniq_key] {
            json::Value::Number(ref n) => {
                if n.is_f64() {
                    n.as_f64().unwrap().into()
                } else {
                    n.as_i64().unwrap().into()
                }
            }
            ref s => s.as_str().unwrap().into(),
        };
        statement
            .bind::<(&str, &Value)>((format!(":{uniq_key}").as_str(), &uniq_val))
            .unwrap();
        while let Ok(State::Row) = statement.next() {
            db_compare.iter().for_each(|&key| {
                if post_as_obj[key].is_number() {
                    match statement.read::<i64, _>(key) {
                        Ok(r) => {
                            matches.insert(key.into(), r.into());
                        }
                        Err(e) => eprintln!("{e:?}"),
                    }
                } else {
                    match statement.read::<String, _>(key) {
                        Ok(r) => {
                            matches.insert(key.into(), r.into());
                        }
                        Err(e) => eprintln!("{e:?}"),
                    }
                }
            })
        }
        let match_len = matches.len();
        if match_len == db_cmp_len {
            if db_compare
                .iter()
                .any(|&key| matches[key] != post_as_obj[key])
            {
                println!("changed: {}", post_as_obj["id"].as_i64().unwrap());
                match retval {
                    Some(r) => {
                        let mut _statement = conn
                            .prepare(
                                format!(
                                    "select {r} from {platform} where {uniq_key} = :{uniq_key}"
                                )
                                .as_str(),
                            )
                            .unwrap();
                        _statement
                            .bind::<&[(&str, &Value)]>(&[(
                                format!(":{uniq_key}").as_str(),
                                &uniq_val,
                            )])
                            .unwrap();
                        if !post_as_obj.contains_key(DUPLICATE) {
                            post_as_obj.insert(DUPLICATE.into(), json::Value::Object(Map::new()));
                        }
                        let duplicate_obj = post_as_obj[DUPLICATE].as_object_mut().unwrap();
                        while let Ok(State::Row) = _statement.next() {
                            match _statement.column_type(r) {
                                Ok(t) => match t {
                                    Type::Integer => {
                                        duplicate_obj.insert(
                                            r.into(),
                                            _statement.read::<i64, _>(r).unwrap().into(),
                                        );
                                    }
                                    _ => {
                                        duplicate_obj.insert(
                                            r.into(),
                                            _statement.read::<String, _>(r).unwrap().into(),
                                        );
                                    }
                                },
                                Err(e) => eprintln!("{e:?}"),
                            }
                        }
                    }
                    None => (),
                }
            } else {
                println!("skip: {}", post_as_obj["id"].as_i64().unwrap());
                *post = json::Value::Null;
            }
        }
    }
    return Some(posts);
}
*/
/*
pub async fn get_json(_get: RequestBuilder) -> serde_json::Value {
    let sleep_dur = Duration::from_secs(1);
    loop {
        match _get.try_clone() {
            Some(c) => match c.send().await {
                Ok(v) => match v.json::<serde_json::Value>().await {
                    Ok(j) => break j,
                    Err(e) => panic!("get_json: {e:?}"),
                },
                Err(e) => {
                    println!("get_json: {e:?}");
                    sleep(sleep_dur).await;
                }
            },
            _ => {
                panic!("unable to clone client");
            }
        }
    }
}
*/
//pub fn send_request(&_client: Client, &_response)
/*
fn _main<'a>(conf: &'a Value, platform: &str) {
    let config: PlatformConfig = match parse_config(conf, platform) {
        Ok(v) => v,
        Err(_) => {
            println!("couldn't parse config!");
            exit(1)
        }
    };
    for tag in config.tags.iter() {
        println!("{}", tag);
    }
}
*/
