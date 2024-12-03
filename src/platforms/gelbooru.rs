use crate::{config::PlatformConfig, worker::Operation};
use reqwest::Client;
use tokio::sync::{mpsc /*, oneshot*/};

#[allow(dead_code)]
pub struct Gelbooru<'g> {
    pub root: &'g str,
    pub config: PlatformConfig,
    pub worker: mpsc::Sender<Operation<'g>>,
    pub client: Client,
}

#[allow(dead_code)]
impl<'g> Gelbooru<'g> {
    pub async fn main(&self) -> () {
        return;
    }
}
