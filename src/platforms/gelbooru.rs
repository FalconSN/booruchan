use crate::{platforms::base::PlatformConfig, worker::Operation};
use reqwest::Client;
use tokio::sync::{mpsc, oneshot};

pub struct Gelbooru<'g> {
    pub root: &'g str,
    pub config: PlatformConfig<'g>,
    pub worker: mpsc::Sender<Operation<'g>>,
    pub client: Client,
}

impl<'g> Gelbooru<'g> {
    pub async fn main(&self) -> () {
        return;
    }
}
