#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

use std::{
    io::{ErrorKind, SeekFrom},
    path::PathBuf,
};

use bytes::BytesMut;
use futures::TryStreamExt;
use reqwest::{
    header::{HeaderMap, HeaderValue, RANGE},
    Client, StatusCode,
};
use tokio::{
    fs::{metadata, File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter},
    time::{sleep, Duration},
};
use tokio_util::io::StreamReader;

use crate::{
    consts::{BLOCKSIZE, GREEN, RESET},
    utils,
};

/*struct ProgressBar<'p> {
    file: &'p PathBuf,
    total_size: &'p usize,
    status: &'p Status,
}

impl<'p> ProgressBar<'p> {
    pub fn new(file: &'p PathBuf, total_size: &'p usize, status: &'p Status) -> Self {
        Self {
            file,
            total_size,
            status,
        }
    }

    pub async fn run(&self) {
        let dur = Duration::from_secs(1);
        while let Status::Running = *self.status {
            match metadata(self.file).await {
                Ok(m) => println!(
                    "{}: {}/{}",
                    self.file.display(),
                    m.st_size(),
                    self.total_size
                ),
                Err(_) => (),
            }
            sleep(dur).await;
        }
        return;
    }
}

#[derive(Clone, Copy)]
enum Status {
    Running,
    Done,
}
*/
pub struct Downloader<'d> {
    pub client: Client,
    pub url: &'d str,
    pub dest: Vec<&'d str>,
    pub fallback: Option<&'d str>,
    pub timeout: Duration,
    pub retries: i64,
    pub retry_sleep: Duration,
    tries: i64,
    written: usize,
    target_size: usize,
    target_file: PathBuf,
    //status: Rc<RefCell<Status>>,
}

impl<'d> Downloader<'d> {
    pub fn new(
        client: Client,
        url: &'d str,
        dest: Vec<&'d str>,
        fallback: Option<&'d str>,
        timeout: Duration,
        retries: i64,
        retry_sleep: Duration,
    ) -> Self {
        Self {
            client,
            url,
            target_file: dest.iter().collect::<PathBuf>(),
            dest,
            fallback,
            timeout,
            retries,
            retry_sleep,
            tries: 0,
            written: 0,
            target_size: 0,
            //status: Rc::new(RefCell::new(Status::Running)),
        }
    }

    async fn resolve_target_file(&mut self) -> () {
        match metadata(&self.target_file).await {
            Ok(metadata) => {
                self.target_size = metadata.st_size() as usize;
                return;
            }
            Err(_) => match self.target_file.parent() {
                Some(parent) => match utils::recursive_dir_create(parent).await {
                    Ok(_) => return,
                    Err(e) => match e.kind() {
                        ErrorKind::PermissionDenied => match self.fallback {
                            Some(fallback) => {
                                if self.dest.len() > 1 {
                                    self.dest[0] = fallback;
                                    self.target_file = self.dest.iter().collect::<PathBuf>();
                                    match utils::recursive_dir_create(&self.target_file).await {
                                        Ok(_) => return,
                                        Err(e) => panic!("{e:?}"),
                                    }
                                } else {
                                    todo!()
                                }
                            }
                            None => panic!("{e:?}"),
                        },
                        _ => panic!("{e:?}"),
                    },
                },
                None => return,
            },
        }
    }

    /*async fn progress_bar(&self) {
        //let target_file: Ref<'_, PathBuf> = _target_file.borrow();
        let dur = Duration::from_secs(1);
        while let Status::Running = self.status {
            println!(
                "{}: {}/{}",
                self.target_file.borrow().display(),
                self.target_size,
                self.total_size
            );
            sleep(self.retry_sleep).await
        }
        return;
    }*/

    pub async fn download(mut self) -> Option<PathBuf> {
        let mut on_download_failure = move || {
            if self.tries >= 0 {
                if self.tries == self.retries {
                    println!("max number of retries reached for '{url}'", url = self.url);
                    return Err(());
                } else {
                    self.tries += 1;
                    return Ok(());
                }
            }
            return Ok(());
        };
        /*let status = Status::Running;
        let handle = {
            let target_file = &target_file;
            let status = &status;
            tokio::task::spawn_local(async move {
                ProgressBar::new(target_file, &self.total_size, status)
                    .run()
                    .await
            })
        };*/
        #[allow(unused_assignments)]
        let mut is_success: bool = false;
        'downloader: loop {
            self.resolve_target_file().await;
            let mut headers = HeaderMap::new();
            if self.target_size > 0 {
                headers.insert(
                    RANGE,
                    HeaderValue::from_str(format!("bytes={}-", self.target_size).as_str()).unwrap(),
                );
            }
            match self
                .client
                .get(self.url)
                .timeout(self.timeout)
                .headers(headers)
                .send()
                .await
            {
                Ok(resp) => {
                    match resp.error_for_status_ref() {
                        Ok(_) => (),
                        Err(e) => match e.status() {
                            Some(status_code) if status_code.is_client_error() => match status_code
                            {
                                StatusCode::RANGE_NOT_SATISFIABLE => {
                                    match resp.headers().get("content-range") {
                                        Some(val) => match std::str::from_utf8(val.as_bytes())
                                            .unwrap()
                                            .rsplit_once('/')
                                        {
                                            Some(range) => match range.1.parse::<usize>() {
                                                Ok(size) => {
                                                    if size == self.target_size {
                                                        is_success = true;
                                                        break 'downloader;
                                                    }
                                                }
                                                Err(_) => panic!(
                                                    "PARSE ERROR: {}\n{e:?}\n{:?}",
                                                    range.0,
                                                    resp.headers()
                                                ),
                                            },
                                            None => panic!("{e:?}\n{:?}", resp.headers()),
                                        },
                                        None => panic!("{e:?}\n{:?}", resp.headers()),
                                    }
                                    is_success = true;
                                    break 'downloader;
                                }
                                _ => panic!("{e:?}\n{:?}", resp.headers()),
                            },
                            Some(_) => eprintln!("{e:?}\n{:?}", resp.headers()),
                            None => todo!(),
                        },
                    }
                    let content_length: usize = match resp.content_length() {
                        Some(s) => s as usize,
                        None => panic!("{resp:?}\n{}", self.url), /*{
                                                                      // konachan does not return content_length header
                                                                      // when bytes == content_length
                                                                      if self.target_size > 0 {
                                                                          is_success = true;
                                                                          break 'downloader;
                                                                      } else {
                                                                          panic!(
                                                                              "target_size: {ts}, target_file: {tf}, url: {url}",
                                                                              ts = self.target_size,
                                                                              tf = self.target_file.display(),
                                                                              url = self.url,
                                                                          );
                                                                      }
                                                                  }*/
                    };
                    //self.total_size = content_length + self.target_size;
                    let file: File = match OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&self.target_file)
                        .await
                    {
                        Ok(f) => f,
                        Err(e) => panic!(
                            "an error occured while opening file: {}\nerror: {:?}",
                            self.target_file.display(),
                            e
                        ),
                    };
                    let mut writer = BufWriter::new(file);
                    writer.seek(SeekFrom::End(0)).await.unwrap();
                    let mut buf: BytesMut = BytesMut::with_capacity(BLOCKSIZE);
                    let stream = resp.bytes_stream();
                    let mut reader =
                        StreamReader::new(stream.map_err(|e| std::io::Error::other(e)));
                    println!(
                        "content_length: {content_length}, target_size: {}\n{}",
                        self.target_size,
                        self.target_file.display(),
                    );
                    loop {
                        match reader.read_buf(&mut buf).await {
                            Ok(r) => {
                                if r == 0 {
                                    if self.written == content_length {
                                        is_success = true;
                                        break 'downloader;
                                        /*println!(
                                            //"\x1b[A\r\x1b[2K{GREEN}{}{RESET}",
                                            "{GREEN}{}{RESET}",
                                            target_file.display()
                                        );
                                        return Some(target_file);*/
                                    } else {
                                        if on_download_failure().is_err() {
                                            return None;
                                        }
                                        sleep(self.retry_sleep).await;
                                        continue 'downloader;
                                    }
                                }
                                self.target_size += writer.write(&mut buf).await.unwrap();
                                writer.flush().await.unwrap();
                                buf.clear();
                            }
                            Err(_) => {
                                if on_download_failure().is_err() {
                                    return None;
                                }
                                sleep(self.retry_sleep).await;
                                continue 'downloader;
                            }
                        }
                    }
                }
                Err(_) => {
                    if on_download_failure().is_err() {
                        return None;
                    }
                    sleep(self.retry_sleep).await;
                    continue 'downloader;
                }
            }
        }
        /*status = Status::Done;
        handle.await.unwrap();*/
        if is_success {
            println!("{GREEN}{}{RESET}", self.target_file.display());
            return Some(self.target_file);
        } else {
            return None;
        }
    }
}
