// std imports
#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
use std::{
    future::Future,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

// crate imports
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};

// local
use crate::consts::BLOCKSIZE;

// used by mvf
async fn hard_move<F, Fu>(src: &str, dest: &str, on_success: F)
where
    F: FnOnce() -> Fu,
    Fu: Future<Output = ()>,
{
    let src_size = match fs::metadata(src).await {
        Ok(m) => m.st_size(),
        Err(e) => panic!("{e:?}"),
    };
    let src_open: File = fs::OpenOptions::new().read(true).open(&src).await.unwrap();
    let dest_open: File = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(dest)
        .await
        .unwrap();
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

pub async fn mvf<F, Fu>(src: &str, dest: &str, on_success: F)
where
    F: FnOnce() -> Fu,
    Fu: Future<Output = ()>,
{
    //#[repr(i32)]
    enum MoveError {
        ENOENT = 1,
        EACCESS = 12,
        EXDEV = 18,
        UNKNOWN = -1,
    }

    impl From<i32> for MoveError {
        fn from(value: i32) -> Self {
            match value {
                2 => Self::ENOENT,
                12 => Self::EACCESS,
                18 => Self::EXDEV,
                _ => Self::UNKNOWN,
            }
        }
    }

    match fs::rename(&src, &dest).await {
        Ok(_) => (),
        Err(e) => match e.raw_os_error() {
            Some(code) => match MoveError::from(code) {
                MoveError::ENOENT => {
                    eprintln!("unable to move, no such file or directory: {src}");
                }
                MoveError::EACCESS => {
                    eprintln!("cannot move, permission denied: {src}");
                }
                MoveError::EXDEV => return hard_move(src, dest, on_success).await,
                _ => panic!("{e:?}"),
            },
            None => panic!("{e:?}"),
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
