use crate::utils;
use sqlite::{Connection, ConnectionThreadSafe, State, Statement, Value};
#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{io::ErrorKind, process::exit};
use tokio::{
    sync::{mpsc, oneshot},
    task,
};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DbEntry {
    pub id: i64,
    pub md5: String,
    pub source: Option<String>,
    pub tags: Option<String>,
    pub path: String,
    pub compress_path: Option<String>,
}

pub struct Insert {
    pub platform: &'static str,
    pub entry: DbEntry,
}

pub struct Select {
    pub platform: &'static str,
    pub id: i64,
    pub sender: oneshot::Sender<Option<DbEntry>>,
}

pub struct ImageRequest {
    pub src: PathBuf,
    pub dest: Vec<String>,
    pub size: (u32, u32),
    pub fallback: Option<String>,
    pub response_channel: oneshot::Sender<Option<PathBuf>>,
}

pub enum Operation {
    Insert(Insert),
    Select(Select),
    Image(ImageRequest),
    Close,
}

pub struct Worker {
    pub buf: mpsc::Receiver<Operation>,
    connection: ConnectionThreadSafe,
}

impl Worker {
    pub fn new<D: AsRef<Path>>(database: D, receiver: mpsc::Receiver<Operation>) -> Self {
        Self {
            buf: receiver,
            connection: Connection::open_thread_safe(database.as_ref()).unwrap(),
        }
    }

    pub async fn main(&mut self) {
        loop {
            match self.buf.recv().await {
                Some(opt) => match opt {
                    Operation::Insert(i) => self.insert(i).await,
                    Operation::Select(s) => self.select(s).await,
                    Operation::Image(r) => {
                        let t = task::spawn_blocking(|| image_resize(r));
                        t.await.unwrap();
                    }
                    Operation::Close => self.buf.close(),
                },
                None => return,
            }
        }
    }

    async fn select(&self, entry: Select) {
        let mut statement = match self.connection.prepare(format!(
            "SELECT * FROM {table} WHERE id = ?",
            table = entry.platform
        )) {
            Ok(mut s) => {
                s.bind((1, entry.id)).unwrap();
                s
            }
            Err(_) => {
                entry.sender.send(None).unwrap();
                return;
            }
        };
        let mut ret = DbEntry::default();
        while let Ok(State::Row) = statement.next() {
            ret.id = statement.read::<i64, _>(0).unwrap();
            ret.md5 = statement.read::<String, _>(1).unwrap();
            ret.source = statement.read::<Option<String>, _>(2).unwrap();
            ret.tags = statement.read::<Option<String>, _>(3).unwrap();
            ret.path = statement.read::<String, _>(4).unwrap();
            ret.compress_path = statement.read::<Option<String>, _>(5).unwrap();
        }
        entry.sender.send(Some(ret)).unwrap();
        return;
    }

    async fn insert(&self, db_entry: Insert) {
        self.connection
            .execute(format!(
                "CREATE TABLE IF NOT EXISTS {table}(
                    id INT PRIMARY KEY,
                    md5 TEXT NOT NULL,
                    source TEXT,
                    tags TEXT,
                    path TEXT NOT NULL,
                    compress_path TEXT)",
                table = db_entry.platform
            ))
            .unwrap();

        let query: String = format!(
            "INSERT OR REPLACE INTO {table} VALUES(?, ?, ?, ?, ?, ?)", /* id, md5, source, tags, path, compress_path*/
            table = db_entry.platform,
        );
        let mut statement: Statement = self.connection.prepare(query).unwrap();
        statement
            .bind_iter::<_, (usize, Value)>([
                (1, Value::Integer(db_entry.entry.id)),
                (2, Value::String(db_entry.entry.md5)),
                (
                    3,
                    match db_entry.entry.source {
                        Some(s) => Value::String(s),
                        None => Value::Null,
                    },
                ),
                (
                    4,
                    match db_entry.entry.tags {
                        Some(s) => Value::String(s),
                        None => Value::Null,
                    },
                ),
                (5, Value::String(db_entry.entry.path)),
                (
                    6,
                    match db_entry.entry.compress_path {
                        Some(p) => Value::String(p),
                        None => Value::Null,
                    },
                ),
            ])
            .unwrap();
        loop {
            match statement.next() {
                Ok(s) => match s {
                    State::Row => (),
                    State::Done => break,
                },
                Err(e) => panic!("{:?}", e),
            }
        }
    }
}

#[allow(unused_must_use)]
fn image_resize(mut request: ImageRequest) {
    use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageReader};
    use std::{fs, io};
    let mut dest_path = request.dest.iter().collect::<PathBuf>();
    match utils::recursive_dir_create_blocking(dest_path.parent().unwrap()) {
        Ok(_) => match fs::metadata(&dest_path) {
            Ok(m) => {
                if m.st_size() > 0 {
                    fs::remove_file(&dest_path).unwrap();
                }
            }
            Err(_) => (),
        },
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => match request.fallback {
                Some(fallback) => {
                    request.dest[0] = fallback;
                    dest_path = request.dest.iter().collect::<PathBuf>();
                    match utils::recursive_dir_create_blocking(dest_path.parent().unwrap()) {
                        Ok(_) => match fs::metadata(&dest_path) {
                            Ok(m) => {
                                if m.st_size() > 0 {
                                    fs::remove_file(&dest_path).unwrap();
                                }
                            }
                            Err(e) => match e.kind() {
                                ErrorKind::NotFound => (),
                                _ => panic!("{e:?}"),
                            },
                        },
                        Err(e) => panic!("{e:?}"),
                    }
                }
                None => {
                    eprintln!(
                        "could not create directory '{}', permission denied.",
                        dest_path.display()
                    );
                    exit(e.raw_os_error().unwrap_or(1));
                }
            },
            _ => panic!("{e:?}"),
        },
    }
    let mut src_image: DynamicImage = match ImageReader::open(&request.src).unwrap().decode() {
        Ok(d) => d,
        Err(_) => {
            eprintln!("decode error for {}", request.src.display());
            request.response_channel.send(None);
            return;
        }
    };
    let src_size: (u32, u32) = (src_image.width(), src_image.height());
    if src_size.0 >= request.size.0 || src_size.1 >= request.size.0 {
        src_image = src_image.resize(request.size.0, request.size.0, FilterType::Lanczos3);
    }
    let dest_file: fs::File = match fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&dest_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error while opening file: {}", dest_path.display());
            panic!("{e:?}");
        }
    };
    let writer: io::BufWriter<fs::File> = io::BufWriter::new(dest_file);
    let encoder: JpegEncoder<io::BufWriter<fs::File>> = JpegEncoder::new_with_quality(writer, 90);
    match src_image.into_rgb8().write_with_encoder(encoder) {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    };
    request.response_channel.send(Some(dest_path));
    return;
}
