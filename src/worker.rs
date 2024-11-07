use crate::utils;
use json::Map;
use serde_json as json;
use sqlite::{Connection, ConnectionThreadSafe, State, Statement, Value};
use std::io::ErrorKind;
#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::{
    //fs,
    sync::{mpsc, oneshot},
    task,
};

pub struct Insert {
    pub unique: String,
    pub table: String,
    pub cols: Map<String, json::Value>,
}

pub struct Select {
    pub query: String,
    pub uniq: String,
    pub bindables: Vec<(usize, Value)>,
    pub sender: oneshot::Sender<Option<Map<String, json::Value>>>,
}

pub struct ImageRequest {
    pub src: PathBuf,
    pub dest: Vec<String>,
    pub size: (u32, u32),
    pub fallback: Option<String>,
    pub response_channel: oneshot::Sender<Option<ImageResponse>>,
}

pub struct ImageResponse {
    pub file: PathBuf,
}

pub enum Operation {
    Insert(Insert),
    Select(Select),
    Image(ImageRequest),
    Close,
}

/*pub struct DbEntry<'d> {
    pub unique: &'d str,
    pub table: &'d str,
    pub cols: Map<String, json::Value>,
}*/

pub struct Worker {
    pub buf: mpsc::Receiver<Operation>,
    connection: ConnectionThreadSafe,
    //sleep: Duration,
}

impl Worker {
    pub fn new<P: AsRef<Path>>(database: P, receiver: mpsc::Receiver<Operation>) -> Self {
        Self {
            buf: receiver,
            connection: match Connection::open_thread_safe(database.as_ref()) {
                Ok(c) => c,
                Err(e) => panic!("{e:?}"),
            },
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
                    Operation::Close => {
                        self.buf.close();
                    }
                },
                None => {
                    return;
                }
            }
        }
    }
    /*
    #[allow(unused_must_use)]
    fn image_resize(&self, mut request: ImageRequest) {
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
                        match utils::recursive_dir_create_blocking(&dest_path) {
                            Ok(_) => (),
                            Err(e) => panic!("{e:?}"),
                        }
                    }
                    None => panic!("{e:?}"),
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
        let encoder: JpegEncoder<io::BufWriter<fs::File>> =
            JpegEncoder::new_with_quality(writer, 90);
        match src_image.into_rgb8().write_with_encoder(encoder) {
            Ok(_) => (),
            Err(e) => panic!("{:?}", e),
        };
        request
            .response_channel
            .send(Some(ImageResponse { file: dest_path }));
        return;
    }
    */
    async fn select(&self, entry: Select) {
        let mut statement = match self.connection.prepare(entry.query) {
            Ok(s) => s,
            Err(_) => {
                entry.sender.send(None).unwrap();
                return;
            }
        };
        let mut map: Map<String, json::Value> = Map::new();
        statement
            .bind_iter::<_, (usize, Value)>(entry.bindables)
            .unwrap();
        while let Ok(State::Row) = statement.next() {
            for column in statement.column_names() {
                let column_str = column.as_str();
                let val = statement.read::<Value, _>(column_str).unwrap();
                map.insert(
                    column_str.to_owned(),
                    match val {
                        Value::String(s) => s.as_str().into(),
                        Value::Integer(i) => i.into(),
                        Value::Null => json::Value::Null,
                        Value::Float(f) => f.into(),
                        _ => panic!(),
                    },
                );
            }
        }
        entry.sender.send(Some(map)).unwrap();
        return;
    }

    async fn insert(&self, db_entry: Insert) {
        self.connection
            .execute(format!(
                "create table if not exists {table}({cols})",
                table = db_entry.table,
                cols = {
                    let mut _cols: Vec<String> = Vec::new();
                    for (k, v) in db_entry.cols.iter() {
                        if v.is_number() {
                            if k.as_str() == db_entry.unique {
                                _cols.push(format!("{k} INT PRIMARY KEY"));
                            } else {
                                _cols.push(format!("{k} INT"));
                            }
                        } else {
                            if k.as_str() == db_entry.unique {
                                _cols.push(format!("{k} TEXT PRIMARY KEY"));
                            } else {
                                _cols.push(format!("{k} TEXT"));
                            }
                        }
                    }
                    _cols.join(", ")
                }
            ))
            .unwrap();
        let query: String = format!(
            "insert or replace into {table} values({values})",
            table = db_entry.table,
            values = {
                let mut _cols: Vec<String> = Vec::new();
                for (k, _) in db_entry.cols.iter() {
                    _cols.push(format!(":{k}"));
                }
                _cols.join(", ")
            }
        );
        let mut statement: Statement = self.connection.prepare(query).unwrap();
        for (k, v) in db_entry.cols.iter() {
            match statement.bind::<&[(_, Value)]>(&[(
                format!(":{k}").as_str(),
                match *v {
                    json::Value::Number(ref n) => n.as_i64().unwrap().into(),
                    json::Value::Null => Value::Null, //panic!("null object found while binding: {}", k),
                    json::Value::String(ref s) => s.as_str().into(),
                    _ => panic!(
                        "error encountered while binding {}, reason: unknown data type.",
                        k /* this shouldn't happen */
                    ),
                },
            )]) {
                Ok(_) => (),
                Err(e) => panic!(
                    "error encountered while binding {}, value: {:?}\n{}",
                    k, v, e
                ),
            }
        }
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
                    match utils::recursive_dir_create_blocking(&dest_path) {
                        Ok(_) => (),
                        Err(e) => panic!("{e:?}"),
                    }
                }
                None => panic!("{e:?}"),
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
    request
        .response_channel
        .send(Some(ImageResponse { file: dest_path }));
    return;
}

/*async fn add_to_db(conn: &Connection, unique: &str, table: &str, vals: Map<String, json::Value>) {
    conn.execute(format!(
        "create table if not exists {table}({cols})",
        cols = {
            let mut _cols: Vec<String> = Vec::new();
            for (k, v) in vals.iter() {
                if v.is_number() {
                    if k.as_str() == unique {
                        _cols.push(format!("{k} INT PRIMARY KEY"));
                    } else {
                        _cols.push(format!("{k} INT"));
                    }
                } else {
                    if k.as_str() == unique {
                        _cols.push(format!("{k} TEXT PRIMARY KEY"));
                    } else {
                        _cols.push(format!("{k} TEXT"));
                    }
                }
            }
            _cols.join(", ")
        }
    ))
    .unwrap();
    let query: String = format!(
        "insert or replace into {table} values({values})",
        values = {
            let mut _cols: Vec<String> = Vec::new();
            for (k, _) in vals.iter() {
                _cols.push(format!(":{k}"));
            }
            _cols.join(", ")
        }
    );
    let mut statement: Statement = conn.prepare(query).unwrap();
    for (k, v) in vals.iter() {
        match statement.bind::<&[(_, Value)]>(&[(
            format!(":{k}").as_str(),
            match *v {
                json::Value::Number(ref n) => n.as_i64().unwrap().into(),
                json::Value::Null => Value::Null, //panic!("null object found while binding: {}", k),
                json::Value::String(ref s) => s.as_str().into(),
                _ => panic!(
                    "error encountered while binding {}, reason: unknown data type.",
                    k /* this shouldn't happen */
                ),
            },
        )]) {
            Ok(_) => (),
            Err(e) => panic!(
                "error encountered while binding {}, value: {:?}\n{}",
                k, v, e
            ),
        }
    }
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

pub async fn _add_to_db<D: AsRef<Path>>(
    database: D,
    unique: &str,
    table: &str,
    vals: Map<String, json::Value>,
) {
    match utils::recursive_file_create(&database).await {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    }
    let conn: ConnectionThreadSafe = match Connection::open_thread_safe(&database) {
        Ok(c) => c,
        Err(e) => panic!("{:?}", e),
    };
    conn.execute(format!(
        "create table if not exists {table}({cols})",
        cols = {
            let mut _cols: Vec<String> = Vec::new();
            for (k, v) in vals.iter() {
                if v.is_number() {
                    if k.as_str() == unique {
                        _cols.push(format!("{k} INT PRIMARY KEY"));
                    } else {
                        _cols.push(format!("{k} INT"));
                    }
                } else {
                    if k.as_str() == unique {
                        _cols.push(format!("{k} TEXT PRIMARY"));
                    } else {
                        _cols.push(format!("{k} TEXT"));
                    }
                }
            }
            _cols.join(", ")
        }
    ))
    .unwrap();
    let query: String = format!(
        "insert or replace into {table} values({values})",
        values = {
            let mut _cols: Vec<String> = Vec::new();
            for (k, _) in vals.iter() {
                _cols.push(format!(":{k}"));
            }
            _cols.join(", ")
        }
    );
    let mut statement: Statement = conn.prepare(query).unwrap();
    for (k, v) in vals.iter() {
        match statement.bind::<&[(_, Value)]>(&[(
            format!(":{k}").as_str(),
            match *v {
                json::Value::Number(ref n) => n.as_i64().unwrap().into(),
                json::Value::Null => Value::Null, //panic!("null object found while binding: {}", k),
                json::Value::String(ref s) => s.as_str().into(),
                _ => panic!(
                    "error encountered while binding {}, reason: unknown data type.",
                    k /* this shouldn't happen */
                ),
            },
        )]) {
            Ok(_) => (),
            Err(e) => panic!(
                "error encountered while binding {}, value: {:?}\n{}",
                k, v, e
            ),
        }
    }
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
*/
