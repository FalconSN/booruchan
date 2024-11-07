pub mod downloader;
pub mod json_utils;
pub mod platforms;
pub mod rclone;
pub mod string;
pub mod utils;
pub mod worker;

pub mod consts {
    pub const NULL: &str = "null";
    pub const YANDERE: &str = "yandere";
    pub const YANDERE_ROOT: &str = "https://yande.re/post.json";
    pub const KONACHAN: &str = "konachan";
    pub const KONACHAN_ROOT: &str = "https://konachan.com/post.json";
    pub const SAKUGABOORU: &str = "sakugabooru";
    pub const SAKUGABOORU_ROOT: &str = "https://sakugabooru.com/post.json";
    pub const GELBOORU: &str = "gelbooru";
    pub const GELBOORU_ROOT: &str = "https://gelbooru.com/index.php?page=dapi&s=post&q=index";

    pub const DUPLICATE: &str = "_duplicate_";
}

pub mod statics {
    use std::env::var;
    use std::sync::LazyLock;
    pub static HOME: LazyLock<String> = LazyLock::new(|| match var("HOME") {
        Ok(v) => v,
        Err(_) => panic!("HOME variable is not set!"),
    });
}
