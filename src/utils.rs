//use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageReader};
use std::future::Future;
use std::io::{Error, ErrorKind};
#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

use crate::downloader::BLOCKSIZE;

pub async fn mvf<F, Fu>(src: &str, dest: &str, on_success: F)
where
    F: FnOnce() -> Fu,
    Fu: Future<Output = ()>,
{
    let src_size: u64 = match fs::metadata(src).await {
        Ok(m) => m.st_size(),
        Err(e) => panic!("{:?}", e),
    };
    match fs::rename(&src, &dest).await {
        Ok(_) => (),
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                println!("unable to move {}\n{}", &src, e);
            }
            _ => {
                let src_open: File = fs::OpenOptions::new().read(true).open(&src).await.unwrap();
                let dest_open: File = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(dest)
                    .await
                    .unwrap();
                //let buf: BytesMut = BytesMut::with_capacity(65537);
                let mut buf: Vec<u8> = Vec::with_capacity(BLOCKSIZE);
                let mut reader: BufReader<File> = BufReader::new(src_open);
                let mut writer: BufWriter<File> = BufWriter::new(dest_open);
                loop {
                    match reader.read(&mut buf).await {
                        Ok(v) => {
                            if v == 0
                                && match fs::metadata(src).await {
                                    Ok(m) => m.st_size(),
                                    Err(e) => panic!("{:?}", e),
                                } == src_size
                            {
                                on_success().await;
                                return;
                            }
                            match writer.write_all(&buf).await {
                                Ok(_) => {
                                    buf.clear();
                                }
                                Err(e) => {
                                    println!("unable to write to file ({}) to move!", src);
                                    panic!("{:?}", e);
                                }
                            }
                            buf.clear();
                        }
                        Err(e) => {
                            println!("unable to read file ({}) to move!", src);
                            panic!("{:?}", e)
                        }
                    }
                }
            }
        },
    }
}

pub async fn recursive_dir_create<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let mut path_build: PathBuf = PathBuf::new();
    for c in path.as_ref().components() {
        path_build.push(c);
        match path_build.metadata() {
            Ok(_) => continue,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => match fs::create_dir(&path_build).await {
                    Ok(_) => (),
                    Err(e) => match e.kind() {
                        ErrorKind::PermissionDenied => return Err(e),
                        _ => panic!("{:?}", e),
                    },
                },
                ErrorKind::PermissionDenied => return Err(e),
                _ => panic!("{:?}", e),
            },
        }
    }
    return Ok(());
}

pub async fn recursive_file_create<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let p = path.as_ref();
    match recursive_dir_create(p.parent().unwrap()).await {
        Ok(_) => (),
        Err(e) => return Err(e),
    };
    match path.as_ref().metadata() {
        Ok(_) => return Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => match fs::File::create(p).await {
                Ok(_) => return Ok(()),
                Err(e) => return Err(e),
            },
            _ => panic!("{:?}", e),
        },
    }
}

pub fn recursive_dir_create_blocking<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    use std::fs;
    let mut path_build: PathBuf = PathBuf::new();
    for c in path.as_ref().components() {
        path_build.push(c);
        match path_build.metadata() {
            Ok(_) => continue,
            Err(e) => match e.kind() {
                ErrorKind::NotFound => match fs::create_dir(&path_build) {
                    Ok(_) => (),
                    Err(e) => match e.kind() {
                        ErrorKind::PermissionDenied => return Err(e),
                        _ => panic!("{:?}", e),
                    },
                },
                ErrorKind::PermissionDenied => return Err(e),
                _ => panic!("{:?}", e),
            },
        }
    }
    return Ok(());
}

pub fn recursive_file_create_blocking<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    use std::fs;
    let p = path.as_ref();
    match recursive_dir_create_blocking(p.parent().unwrap()) {
        Ok(_) => (),
        Err(e) => return Err(e),
    };
    match path.as_ref().metadata() {
        Ok(_) => return Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => match fs::File::create(p) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(e),
            },
            _ => panic!("{:?}", e),
        },
    }
}

/*pub async fn image_resize<'image, F: FnOnce(), P: AsRef<Path>>(
    _src: P,
    mut dest: Vec<&'image str>,
    size: &(u32, u32),
    fallback: Option<&'image str>,
    on_success: F,
) -> Option<PathBuf> {
    let src: &Path = _src.as_ref();
    let mut dest_path: PathBuf = dest.iter().collect::<PathBuf>();
    match recursive_dir_create(dest_path.parent().unwrap()).await {
        Ok(_) => {
            match fs::metadata(&dest_path).await {
                Ok(m) => {
                    if m.st_size() > 0 {
                        //println!("file already exists, skipping: {}", dest_path.display());
                        fs::remove_file(&dest_path).await.unwrap();
                    }
                }
                Err(_) => (),
            }
        }
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => match fallback {
                Some(f) => {
                    dest[0] = f;
                    dest_path = dest.iter().collect::<PathBuf>();
                    match recursive_dir_create(dest_path.parent().unwrap()).await {
                        Ok(_) => (),
                        Err(e) => panic!("{:?}", e),
                    }
                }
                None => panic!("{:?}", e),
            },
            _ => panic!("{:?}", e),
        },
    }
    let mut src_image: DynamicImage = match ImageReader::open(src).unwrap().decode() {
        Ok(d) => d,
        Err(_) => {
            println!("decode error for {}", src.display());
            return None;
        }
    };
    let src_size: (u32, u32) = (src_image.width(), src_image.height());
    if src_size.0 >= size.0 || src_size.1 >= size.0 {
        println!("resizing...");
        src_image = src_image.resize(size.0, size.0, FilterType::Lanczos3);
        println!("resized.");
    }
    let dest_file: std::fs::File = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&dest_path)
    {
        Ok(f) => f,
        Err(e) => {
            println!("error while opening file: {}", dest_path.display());
            panic!("{:?}", e);
        }
    };
    let writer: std::io::BufWriter<std::fs::File> = std::io::BufWriter::new(dest_file);
    let encoder: JpegEncoder<std::io::BufWriter<std::fs::File>> =
        JpegEncoder::new_with_quality(writer, 90);
    println!("converting...");
    match src_image.into_rgb8().write_with_encoder(encoder) {
        Ok(_) => {
            println!("converted.");
            on_success();
        }
        Err(e) => panic!("{:?}", e),
    };
    return Some(dest_path);
    /*
    match src_image.save_with_format(dest, ImageFormat::Jpeg) {
        Ok(_) => {
            println!("executing on_success");
            on_success();
        }
        Err(e) => {
            println!("couldn't compress {}!\n{}", src.to_str().unwrap(), e);
            panic!();
        }
    }
    */
}*/
