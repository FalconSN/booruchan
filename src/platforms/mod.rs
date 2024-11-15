pub mod base;
mod gelbooru;
mod moebooru;
//pub mod gelbooru;

pub use base::{Platform, PlatformConfig};
pub use gelbooru::Gelbooru;
pub use moebooru::{MoePost, Moebooru};
