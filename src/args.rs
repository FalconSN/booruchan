use crate::statics::HOME;
/*use std::env;
use std::path::PathBuf;
use tokio::fs;*/
use std::{env, fs, path::PathBuf};

pub struct FileArg {
    pub path: PathBuf,
    pub is_custom: bool,
}

pub struct Args {
    pub config: FileArg,
    pub database: FileArg,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            config: FileArg {
                path: config_default(),
                is_custom: false,
            },
            database: FileArg {
                path: [HOME.as_str(), ".archives", "booruchan.db"]
                    .iter()
                    .collect::<PathBuf>(),
                is_custom: false,
            },
        }
    }
}

impl Args {
    pub fn parse() -> Self {
        use std::process::exit;

        let mut args = Args::default();
        let _args_ = env::args().collect::<Vec<String>>();
        let mut iter = env::args();
        iter.next(); // consume argv[0]
        let mut i: usize = 1;
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--database" | "-d" => match _args_.get(i + 1) {
                    Some(path) => {
                        let buf = PathBuf::from(path);
                        match fs::OpenOptions::new().read(true).open(&buf) {
                            Ok(_) => {
                                args.database = FileArg {
                                    path: buf,
                                    is_custom: true,
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "unable to read database file: {}\nerror: {e:?}",
                                    buf.display()
                                );
                                exit(2);
                            }
                        }
                        iter.next();
                        i += 2;
                    }
                    None => {
                        eprintln!("arg '{}' is used but no path specified.", arg.as_str());
                        exit(1);
                    }
                },
                "--config" | "-c" => match _args_.get(i + 1) {
                    Some(path) => {
                        let buf = PathBuf::from(path);
                        match fs::OpenOptions::new().read(true).open(&buf) {
                            Ok(_) => {
                                args.config = FileArg {
                                    path: buf,
                                    is_custom: true,
                                };
                            }
                            Err(e) => {
                                eprintln!(
                                    "unable to read config file: {}\nerror: {e:?}",
                                    buf.display()
                                );
                                exit(2)
                            }
                        }
                        iter.next();
                        i += 2;
                    }
                    None => panic!("arg '{}' used but no path specified.", arg.as_str()),
                },
                "--" => break,
                _ => panic!("unexpected argument: {}", arg),
            }
        }
        return args;
    }
}

fn config_default() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        [xdg.as_str(), "booruchan", "booruchan.json"]
            .iter()
            .collect::<PathBuf>()
    } else {
        [HOME.as_str(), ".config", "booruchan", "booruchan.json"]
            .iter()
            .collect::<PathBuf>()
    }
}
