use bytes::BytesMut;
use futures::TryStreamExt;
use reqwest::{
    header::{HeaderMap, HeaderValue, RANGE},
    Client,
};
#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
//use std::os::unix::fs::MetadataExt;
use std::{
    io::{ErrorKind, SeekFrom},
    path::PathBuf,
};
use tokio::{
    fs::{metadata, File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter},
    time::{sleep, Duration},
};
use tokio_util::io::StreamReader;

use crate::utils;

pub const BLOCKSIZE: usize = 1048576;
pub const GREEN: &str = "\x1b[32;1;1m";
pub const RESET: &str = "\x1b[0m";

/*
async fn progress<F: AsRef<Path>>(file: F, size: u64) {
    let file: &Path = file.as_ref();
    let sleep_sec: Duration = Duration::new(1, 0);
    loop {
        match fs::metadata(file).await {
            Ok(s) => print!("{} {}/{}", file.display(), s.st_size(), size),
            Err(e) => panic!("{:?}", e),
        }
        sleep(sleep_sec).await;
    }
}
*/

pub async fn main<'download>(
    client: &Client,
    url: &str,
    mut dest: Vec<&str>,
    fallback: Option<&'download str>,
    timeout: Duration,
    retries: i64,
    retry_sleep: Duration,
) -> Option<PathBuf> {
    let mut target_file: PathBuf = dest.iter().collect::<PathBuf>();
    match utils::recursive_dir_create(target_file.parent().unwrap()).await {
        Ok(_) => (),
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => match fallback {
                Some(f) => {
                    dest[0] = f;
                    target_file = dest.iter().collect::<PathBuf>();
                    match utils::recursive_dir_create(target_file.parent().unwrap()).await {
                        Ok(_) => (),
                        Err(e) => panic!("{:?}", e),
                    }
                }
                None => {
                    println!(
                        "error while creating directory: {}",
                        target_file.parent().unwrap().display()
                    );
                    panic!("{e:?}")
                }
            },
            _ => panic!("{e:?}"),
        },
    };
    let mut tries: i64 = 0;
    'downloader: loop {
        let target_size: u64 = match metadata(&target_file).await {
            Ok(m) => m.st_size(),
            Err(e) => match e.kind() {
                ErrorKind::NotFound => 0,
                _ => panic!("{:?}", e),
            },
        };
        let mut headers = HeaderMap::new();
        if target_size > 0 {
            headers.insert(
                RANGE,
                HeaderValue::from_str(format!("bytes={}-", target_size).as_str()).unwrap(),
            );
        }
        match client
            .get(url)
            .timeout(timeout)
            .headers(headers)
            .send()
            .await
        {
            Ok(resp) => {
                //let content_length: usize = resp.content_length().unwrap() as usize;
                let content_length: usize = match resp.content_length() {
                    Some(s) => s as usize,
                    None => {
                        if target_size > 0 {
                            println!("{GREEN}{}{RESET}", target_file.display());
                            return Some(target_file);
                        } else {
                            panic!(
                                "target_size: {target_size}, target_file: {tf}, url: {url}",
                                tf = target_file.display()
                            );
                        }
                    }
                };
                if content_length == 0 {
                    println!("{GREEN}{}{RESET}", target_file.display());
                    return Some(target_file);
                }
                let file: File = match OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&target_file)
                    .await
                {
                    Ok(f) => f,
                    Err(e) => panic!(
                        "an error occured while opening file: {}\n{}",
                        target_file.display(),
                        e
                    ),
                };
                let mut writer = BufWriter::new(file);
                if target_size > 0 {
                    writer.seek(SeekFrom::End(0)).await.unwrap();
                }
                let mut buf: BytesMut = BytesMut::with_capacity(BLOCKSIZE);
                let stream = resp.bytes_stream();
                let mut reader = StreamReader::new(stream.map_err(|e| std::io::Error::other(e)));
                println!(
                    "content_length: {content_length}, target_size: {target_size}\n{}",
                    target_file.display()
                );
                let mut written: usize = 0;
                loop {
                    match reader.read_buf(&mut buf).await {
                        Ok(r) => {
                            if r == 0 {
                                if written == content_length {
                                    println!(
                                        //"\x1b[A\r\x1b[2K{GREEN}{}{RESET}",
                                        "{GREEN}{}{RESET}",
                                        target_file.display()
                                    );
                                    return Some(target_file);
                                } else {
                                    continue 'downloader;
                                }
                            }
                            written += writer.write(&mut buf).await.unwrap();
                            writer.flush().await.unwrap();
                            buf.clear();
                        }
                        Err(_) => {
                            if tries == retries {
                                println!("max number of retries reached for {}", url);
                                return None;
                            }
                            tries += 1;
                            sleep(retry_sleep).await;
                            continue 'downloader;
                        }
                    }
                }
            }
            Err(_) => {
                if tries == retries {
                    println!("max number of retries reached for {}", url);
                    return None;
                }
                tries += 1;
                sleep(retry_sleep).await;
                continue 'downloader;
            }
        }
    }
}
