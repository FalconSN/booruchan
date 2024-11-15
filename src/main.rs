use booruchan::{
    consts::*,
    platforms::base::{parse_config, Platform, PlatformConfig},
    worker::{Operation, Worker},
    HOME,
};
use reqwest::ClientBuilder;
use serde_json::{Map, Value};
use std::{
    path::{Path, PathBuf},
    process::exit,
};
use tokio::{
    fs,
    sync::mpsc,
    task::{self, JoinSet},
    time::Duration,
};

async fn get_config() -> Map<String, Value> {
    let config_dir = Path::new(HOME.as_str()).join(".config").join("booruchan");
    if !config_dir.is_dir() {
        match fs::create_dir_all(&config_dir).await {
            Ok(_) => (),
            Err(_) => {
                println!("unable to create config directory!");
                exit(1);
            }
        }
    }
    let config_file = Path::new(&config_dir).join("booruchan.json");
    if !config_file.is_file() {
        println!("config file doesn't exist: {}", config_file.display());
        exit(1)
    }
    let _read: String = match fs::read_to_string(&config_file).await {
        Ok(v) => v,
        Err(_) => {
            println!("Unable to read config file!");
            exit(1);
        }
    };
    let config: Map<String, Value> = match serde_json::from_str(&_read.as_str()) {
        Ok(v) => v,
        Err(_) => {
            println!("cannot parse config file!");
            exit(1);
        }
    };
    return config;
}

#[tokio::main]
async fn main() {
    let config: &mut Map<String, Value> = Box::leak(Box::new(get_config().await));
    let platforms = [YANDERE, KONACHAN, SAKUGABOORU, GELBOORU];
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent("curl/8.10.1")
        .build()
        .unwrap();
    let mut set: JoinSet<()> = JoinSet::new();
    let (sender, receiver) = mpsc::channel(10);
    let mut worker = Worker::new(
        [HOME.as_str(), ".archives", "booruchan.db"]
            .iter()
            .collect::<PathBuf>(),
        receiver,
    );
    let worker_handle = task::spawn(async move { worker.main().await });
    for platform in platforms {
        if config.contains_key(platform) {
            let _sender = sender.clone();
            let _client = client.clone();
            let conf: PlatformConfig = parse_config(config, platform).unwrap();
            set.spawn(async move { Platform::init(conf, _sender, _client).await });
        }
    }
    set.join_all().await;
    sender.send(Operation::Close).await.unwrap();
    worker_handle.await.unwrap();
}
