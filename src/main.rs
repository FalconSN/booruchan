// std imports
use std::{path::Path, process::exit};

// local imports
use booruchan::{init_platforms, Args, Config};

async fn parse_config(args: &Args) -> Config {
    use std::io::ErrorKind;
    use tokio::{fs, io::AsyncReadExt};
    let path: &Path = args.config.path.as_ref();
    let mut file_content: String = String::new();
    let conf: Config;
    match fs::OpenOptions::new().read(true).open(path).await {
        Ok(mut f) => {
            f.read_to_string(&mut file_content).await.unwrap();
            conf = serde_json::from_str(file_content.as_str()).unwrap();
            return conf;
        }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                if args.config.is_custom {
                    eprintln!("config file not found: {}", path.display());
                    exit(2)
                }
                conf = serde_json::from_str("{}").unwrap();
                return conf;
            }
            _ => panic!("{e:?}"),
        },
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse().await;
    let conf = parse_config(&args).await;
    init_platforms(conf, args).await;
}
