pub mod base;
mod gelbooru;
mod moebooru;

pub use moebooru::Moebooru;

pub mod statics {
    pub static YANDERE: &str = "yandere";
    pub static YANDERE_ROOT: &str = "https://yande.re/post.json";
    pub static KONACHAN: &str = "konachan";
    pub static KONACHAN_ROOT: &str = "https://konachan.com/post.json";
    pub static SAKUGABOORU: &str = "sakugabooru";
    pub static SAKUGABOORU_ROOT: &str = "https://sakugabooru.com/post.json";
    pub static GELBOORU: &str = "gelbooru";
    pub static GELBOORU_ROOT: &str = "https://gelbooru.com/index.php?page=dapi&s=post&q=index";
}
