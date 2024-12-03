// local imports
use booruchan::{statics::ARGS, worker::Operation, worker::Worker, Config};

// crate
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

#[tokio::main]
async fn main() {
    let conf = Config::load();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("booruchan/0.1.0")
        .build()
        .unwrap();
    let mut set: JoinSet<()> = JoinSet::new();
    let (sender, receiver): (Sender<Operation>, Receiver<Operation>) = mpsc::channel(10);
    let mut worker = Worker::new(&ARGS.database.path, receiver);
    let worker_handle = tokio::spawn(async move {
        worker.main().await;
    });
    for p in conf.platforms {
        let client = client.clone();
        let sender = sender.clone();
        {
            set.spawn(async move {
                p.init(client, sender).await;
            })
        };
    }
    set.join_all().await;
    sender.send(Operation::Close).await.unwrap();
    worker_handle.await.unwrap();
    return;
}
