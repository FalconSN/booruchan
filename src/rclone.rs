use std::future::Future;
use std::io;
use std::process::{exit, Stdio};
use tokio::fs;
use tokio::io::{stderr, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/*
#[derive(Debug)]
pub enum RcloneErrorKind {
    MaxRetriesExceeded,
    Move,
}

#[derive(Debug)]
#[allow(dead_code)]
struct RcloneError {
    kind: RcloneErrorKind,
    msg: String,
}

impl std::fmt::Display for RcloneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for RcloneError {}
*/

pub async fn upload<'up, F, Fu, P>(src: P, _dest: P, delete: bool, on_success: F) -> bool
where
    F: FnOnce() -> Fu,
    Fu: Future<Output = ()>,
    P: AsRef<str>,
{
    let dest: &str = _dest.as_ref();
    let mut errors: u8 = 0;
    let src_str: &str = src.as_ref();
    loop {
        match Command::new("rclone")
            .args(["copy", src_str, dest, "--no-traverse"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                let mut _stderr = child.stderr.take().unwrap();
                match child.wait().await {
                    Ok(s) => {
                        if s.success() {
                            on_success().await;
                            if delete {
                                fs::remove_file(src_str).await.unwrap();
                            }
                            return true;
                        } else {
                            if errors > 5 {
                                /*return Err(RcloneError {
                                    kind: RcloneErrorKind::MaxRetriesExceeded,
                                    msg: format!("max retries exceeded for '{}'", src_str),
                                });*/
                                println!("giving up on uploading {}", src_str);
                                return false;
                            }
                            let mut err: String = String::new();
                            _stderr.read_to_string(&mut err).await.unwrap();
                            println!("upload error: {}", err);
                            errors += 1;
                        }
                    }
                    Err(e) => panic!("{:?}", e),
                }
            }
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => {
                    println!("do you have rclone installed?");
                    exit(1);
                }
                io::ErrorKind::PermissionDenied => {
                    println!("unable to execute rclone!");
                    exit(1);
                }
                _ => panic!("{:?}", e),
            },
        };
    }
}

pub async fn moveto<F, Fu>(src: String, dest: String, on_success: F) -> bool
where
    F: FnOnce() -> Fu,
    Fu: Future<Output = ()>,
{
    loop {
        match Command::new("rclone")
            .args([
                "moveto",
                src.as_str(),
                dest.as_str(),
                "--retries",
                "3",
                "--no-traverse",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                let mut _stderr = child.stderr.take().unwrap();
                match child.wait().await {
                    Ok(exit) => {
                        if exit.success() {
                            println!("successfully moved {src} to {dest}");
                            on_success().await;
                            return true;
                        } else {
                            let mut err = String::new();
                            _stderr.read_to_string(&mut err).await.unwrap();
                            stderr().write_all(err.as_bytes()).await.unwrap();
                            stderr().flush().await.unwrap();
                        }
                    }
                    Err(e) => panic!("{:?}", e),
                }
            }
            Err(e) => panic!("{:?}", e),
        }
    }
}
