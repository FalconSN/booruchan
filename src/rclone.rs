use std::future::Future;
use std::io::ErrorKind;
//use std::path::Path;
//use std::process::{exit, Command, Output};
use std::process::{exit, Stdio};
use tokio::fs;
use tokio::io::{stderr, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

pub async fn upload<'up, F, Fu, P>(src: P, dest: P, delete: bool, on_success: F) -> bool
where
    F: FnOnce() -> Fu,
    Fu: Future<Output = ()>,
    P: AsRef<str>,
{
    let _dest: &str = dest.as_ref();
    let mut errors: u8 = 0;
    let src_str: &str = src.as_ref();
    loop {
        match Command::new("rclone")
            .args(["copy", src_str, _dest, "--no-traverse"])
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
                } /*
                  on_success().await;
                  if delete {
                      fs::remove_file(src_str).await.unwrap();
                  }
                  return true;*/
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    println!("do you have rclone installed?");
                    exit(1);
                }
                ErrorKind::PermissionDenied => {
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
    /*
    let proc: io::Result<Child> = Command::new("rclone")
        .args([
            "moveto",
            src.as_str(),
            dest.as_str(),
            "--retries",
            "1",
            "--no-traverse",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();

    match proc {
        Ok(p) => {
            match p.wait().await {
                Ok(exit) => {
                    if exit.success() {
                        println!("successfully moved {src} to {dest}");
                        on_success().await;
                        return true;
                    } else {
                    }
                }
            }
            println!("successfully moved {src} to {dest}");
            on_success().await;
            return true;
        }
        Err(e) => {
            println!("unable to move {}, error: {}", src, e);
            return false;
        }
    }
    */
}
