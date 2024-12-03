use std::future::Future;

// crate
use reqwest::Client;
use tokio::sync::mpsc;

// local
use super::{/*Gelbooru, */ Moebooru};
use crate::{platforms::statics::*, pub_struct, worker::Operation, PlatformConfig};

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

pub enum Platform {
    Yandere(PlatformConfig),
    Sakugabooru(PlatformConfig),
    Konachan(PlatformConfig),
    //Gelbooru(PlatformConfig),
}

impl Platform {
    pub fn init(self, client: Client, worker: mpsc::Sender<Operation>) -> impl Future<Output = ()> {
        match self {
            Platform::Yandere(config) => {
                Moebooru::new(YANDERE, YANDERE_ROOT, config, worker, client).main()
            }
            Platform::Konachan(config) => {
                Moebooru::new(KONACHAN, KONACHAN_ROOT, config, worker, client).main()
            }
            Platform::Sakugabooru(config) => {
                Moebooru::new(SAKUGABOORU, SAKUGABOORU_ROOT, config, worker, client).main()
            }
        }
    }
}
