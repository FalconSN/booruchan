// crate
//use json::Map;
use reqwest::Client;
//use serde::Deserialize;
//use serde_json as json;
use tokio::{
    sync::mpsc,
    task::{self, JoinSet},
    time::Duration,
};

// local
use super::{/*Gelbooru, */ Moebooru};
use crate::{
    pub_struct,
    worker::{Operation, Worker},
    Args, Config,
};

pub_struct!(TagMap {
    general: Vec<String>,
    character: Vec<String>,
    copyright: Vec<String>,
    artist: Vec<String>,
    metadata: Vec<String>,
    circle: Vec<String>,
    faults: Vec<String>,
    style: Vec<String>,
});

impl TagMap {
    pub fn new() -> Self {
        Self {
            general: Vec::new(),
            character: Vec::new(),
            copyright: Vec::new(),
            artist: Vec::new(),
            metadata: Vec::new(),
            circle: Vec::new(),
            faults: Vec::new(),
            style: Vec::new(),
        }
    }
}

/*#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]*/
pub enum Platform {
    Yandere,
    Sakugabooru,
    Konachan,
    Gelbooru,
}

pub async fn init_platforms(config: Config, args: Args) {
    let client: Client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("curl/8.10.1")
        .build()
        .unwrap();
    let mut set: JoinSet<()> = JoinSet::new();
    let (sender, receiver) = mpsc::channel(10);
    let mut worker: Worker = Worker::new(args.database.path, receiver);
    let worker_handle = task::spawn(async move { worker.main().await });
    match config.yandere {
        Some(conf) => {
            let (worker, client) = (sender.clone(), client.clone());
            set.spawn(async move {
                Moebooru::new(Platform::Yandere, conf, worker, client)
                    .main()
                    .await
            });
        }
        None => (),
    }
    match config.sakugabooru {
        Some(conf) => {
            let (worker, client) = (sender.clone(), client.clone());
            set.spawn(async move {
                Moebooru::new(Platform::Sakugabooru, conf, worker, client)
                    .main()
                    .await
            });
        }
        None => (),
    }
    match config.konachan {
        Some(conf) => {
            let (worker, client) = (sender.clone(), client.clone());
            set.spawn(async move {
                Moebooru::new(Platform::Konachan, conf, worker, client)
                    .main()
                    .await
            });
        }
        None => (),
    }
    set.join_all().await;
    sender.send(Operation::Close).await.unwrap();
    worker_handle.await.unwrap();
    return;
}
