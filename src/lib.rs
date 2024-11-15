//pub mod downloader;
pub mod json_utils;
pub mod platforms;
pub mod rclone;
//pub mod string;
pub mod utils;
pub mod worker;
pub use args::Args;
pub use fmt::{KeywordMap, Value};
pub use statics::HOME;

pub mod consts {
    pub const NULL: &str = "null";
    pub const YANDERE: &str = "yandere";
    pub const YANDERE_ROOT: &str = "https://yande.re/post.json";
    pub const KONACHAN: &str = "konachan";
    pub const KONACHAN_ROOT: &str = "https://konachan.com/post.json";
    pub const SAKUGABOORU: &str = "sakugabooru";
    pub const SAKUGABOORU_ROOT: &str = "https://sakugabooru.com/post.json";
    pub const GELBOORU: &str = "gelbooru";
    pub const GELBOORU_ROOT: &str = "https://gelbooru.com/index.php?page=dapi&s=post&q=index";

    pub const BLOCKSIZE: usize = 1048576;
    pub const GREEN: &str = "\x1b[32;1;1m";
    pub const RESET: &str = "\x1b[0m";

    //pub const DUPLICATE: &str = "_duplicate_";
}

mod statics {
    use std::env::var;
    use std::sync::LazyLock;
    pub static HOME: LazyLock<String> = LazyLock::new(|| match var("HOME") {
        Ok(v) => v,
        Err(_) => panic!("HOME variable is not set!"),
    });
}

mod args {
    use crate::HOME;
    use std::env;
    use std::path::PathBuf;
    use tokio::fs;

    pub struct Args {
        pub config_path: PathBuf,
    }
    impl Args {
        pub async fn parse(&mut self) -> Self {
            let args = env::args().collect::<Vec<String>>();
            let mut arg_iter = args.iter();
            let mut i: usize = 0;
            arg_iter.next();
            let mut config_path: Option<PathBuf> = None;
            while let Some(arg) = arg_iter.next() {
                match arg.as_str() {
                    "--config" | "-c" => match args.iter().nth(i + 1) {
                        Some(path) => {
                            let buf = PathBuf::from(path);
                            match fs::OpenOptions::new()
                                .read(true)
                                .create(false)
                                .open(&buf)
                                .await
                            {
                                Ok(_) => config_path = Some(buf),
                                Err(e) => panic!("unable to open file: {}\n{e:?}", buf.display()),
                            }
                            arg_iter.next();
                            i += 2;
                        }
                        None => panic!("arg '{}' used but no path specified.", arg.as_str()),
                    },
                    "--" => break,
                    _ => panic!("unexpected argument: {}", arg),
                }
            }
            if config_path.is_none() {
                config_path = Some(config_default());
            }
            return Self {
                config_path: config_path.unwrap(),
            };
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
}

#[allow(dead_code)]
mod config {
    use std::path::PathBuf;

    use tokio::fs;
    use tokio::io::AsyncReadExt;

    //use crate::platforms::Platform;
    use crate::HOME;

    pub struct GlobalConfig {
        to_cloud: bool,
        delete: bool,
        cloud: Option<String>,
        database: String,
        db_unique: String,
        db_keys: Vec<String>,
        base_dir: String,
        output_dir: Option<String>,
        filename: String,
        compress: bool,
        compress_db: Option<String>,
        compress_base: Option<String>,
        compress_subdir: Option<String>,
        compress_filename: Option<String>,
        compress_size: Option<(u64, u64)>,
        skip: bool,
        sleep: u64,
        retries: i64,
        retry_sleep: u64,
        timeout: u64,
        fname_repl: Vec<String>,
        dname_repl: Vec<String>,
        yandere: PlatformConfig,
        gelbooru: PlatformConfig,
    }

    pub struct PlatformConfig {
        to_cloud: bool,
        delete: bool,
        cloud: Option<String>,
        database: String,
        db_unique: String,
        db_keys: Vec<String>,
        base_dir: String,
        output_dir: Option<String>,
        filename: String,
        compress: bool,
        compress_db: Option<String>,
        compress_base: Option<String>,
        compress_subdir: Option<String>,
        compress_filename: Option<String>,
        compress_size: Option<(u64, u64)>,
        skip: bool,
        sleep: u64,
        retries: i64,
        retry_sleep: u64,
        timeout: u64,
        fname_repl: Vec<String>,
        dname_repl: Vec<String>,
        tags: Vec<String>,
    }

    impl PlatformConfig {
        pub async fn parse<P: AsRef<PathBuf>>(_path: P) {
            let path = _path.as_ref();
            let mut file = match fs::OpenOptions::new()
                .read(true)
                .create(false)
                .open(path)
                .await
            {
                Ok(f) => f,
                Err(e) => panic!("{e:?}"),
            };
            let mut file_content: String = String::new();
            let _ = file.read_to_string(&mut file_content).await;
        }
    }

    impl Default for PlatformConfig {
        fn default() -> Self {
            Self {
                to_cloud: false,
                delete: false,
                cloud: None,
                database: format!("{}/.archives/booruchan.db", HOME.as_str()),
                db_unique: "id".to_string(),
                db_keys: ["id", "md5", "source", "tags", "filename"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                base_dir: format!("{}/booruchan", HOME.as_str()),
                output_dir: Some("{platform}".to_string()),
                filename: "{id}.{file_ext}".to_string(),
                compress: false,
                compress_db: None,
                compress_base: None,
                compress_subdir: None,
                compress_filename: None,
                compress_size: None,
                skip: true,
                sleep: 1,
                retries: 5,
                retry_sleep: 1,
                timeout: 30,
                fname_repl: [":", "!", "?", "*", "\"", "'", "/"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                dname_repl: [":", "!", "?", "*", "\"", "'"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                tags: Vec::new(),
            }
        }
    }
}

mod fmt {
    use crate::consts::NULL;

    const ARR: [char; 2] = ['[', ']'];
    const VALUE_SEP: char = ',';
    const RANGE_SEP: char = ':';
    enum Index {
        Range(i64, i64),
        Select(i64),
    }
    struct Key<'s> {
        substr: &'s str,
        index: Vec<Index>,
    }
    pub enum Value<'val> {
        String(&'val str),
        Array(Vec<&'val str>),
        Signed(i64),
    }
    pub struct KeywordMap<'k> {
        pub platform: &'k str,
        pub id: i64,
        pub tags: &'k str,
        pub source: &'k str,
        pub md5: &'k str,
        pub file_size: i64,
        pub file_ext: &'k str,
        pub rating: &'k str,
        pub path: &'k str,
        pub general: Vec<&'k str>,
        pub character: Vec<&'k str>,
        pub copyright: Vec<&'k str>,
        pub artist: Vec<&'k str>,
        pub metadata: Vec<&'k str>,
        pub circle: Vec<&'k str>,
        pub faults: Vec<&'k str>,
    }
    impl<'k> KeywordMap<'k> {
        async fn parse_key(&self, substr: &'k str) -> Key {
            let mut key = Key {
                substr: substr,
                index: Vec::new(),
                //select: Vec::new(),
            };
            let split = substr
                .rsplitn(3, ARR)
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>();
            let split_len = split.len();
            if split_len == 1 && split[0] == substr {
                return key;
            } else if split_len == 2 {
                key.substr = split[1];
            } else if split_len > 2 {
                panic!("error while parsing {}", key.substr);
            }
            /*
                split index values by splitting them with ','
                and split ranges with ':'
            */
            let multi_index = split[0].split(VALUE_SEP);
            for index in multi_index {
                //println!("{index:?}");
                if index.contains(RANGE_SEP) {
                    if index.starts_with(RANGE_SEP) {
                        // index is [:], we'll consider it as it as [0:-1] which means from start to end
                        // but should we when there are possibly more indexes separated with ','?
                        if index.len() == 1 {
                            key.index.push(Index::Range(0, -1));
                            continue;
                        }
                        // not sure if this is supposed to return 1 or 2 items
                        let end = index.rsplitn(1, RANGE_SEP).collect::<Vec<&str>>();
                        if end.len() == 1 {
                            key.index
                                .push(Index::Range(0, end[0].parse::<i64>().unwrap()));
                        } else {
                            // I don't know what error we'll have yet
                            panic!("end: {end:?}");
                        }
                    } else if index.ends_with(RANGE_SEP) {
                        // case: [i:]
                        // we'll consider it as [i:-1]
                        let start = index.rsplitn(1, RANGE_SEP).collect::<Vec<&str>>();
                        if start.len() == 1 {
                            key.index
                                .push(Index::Range(start[0].parse::<i64>().unwrap(), -1));
                        } else {
                            panic!("start: {start:?}");
                        }
                    } else {
                        // this is a proper case of slicing
                        // [start:end]
                        let start_end = index.splitn(2, RANGE_SEP).collect::<Vec<&str>>();
                        let start = start_end[0].parse::<i64>().unwrap();
                        let end = start_end[1].parse::<i64>().unwrap();
                        if start > end {
                            panic!(
                                "starting index can't be higher than end in ranges: {start}:{end}"
                            );
                        }
                        key.index.push(Index::Range(start, end));
                    }
                } else {
                    // single index, [index]
                    let to_i64 = index.parse::<i64>().unwrap();
                    key.index.push(Index::Select(to_i64));
                }
            }
            return key;
        }
        async fn get_indexes(&self, fmt_str: &str) -> Vec<(usize, usize)> {
            let mut in_brackets = false;
            let mut indexes: Vec<(usize, usize)> = Vec::new();
            let mut start_index: usize = 0;
            for (index, char) in fmt_str.char_indices() {
                match char {
                    '{' => {
                        if index > 0 {
                            if fmt_str.chars().nth(index - 1).is_some_and(|c| c != '\\') {
                                in_brackets = true;
                                start_index = index;
                            }
                        } else {
                            in_brackets = true;
                            start_index = index;
                        }
                    }
                    '}' => {
                        if index > 0 {
                            if fmt_str.chars().nth(index - 1).is_some_and(|c| c != '\\')
                                && in_brackets
                            {
                                in_brackets = false;
                                indexes.push((start_index, index));
                            }
                        } else if in_brackets {
                            in_brackets = false;
                            indexes.push((start_index, index));
                        }
                    }
                    _ => (),
                }
            }
            return indexes;
        }

        async fn slice_string(&self, index: &Index, string: &str) -> String {
            match *index {
                Index::Range(start, end) => {
                    let str_len = string.len();
                    if start == -1 {
                        match string.chars().last() {
                            Some(s) => s.to_string(),
                            None => NULL.to_string(),
                        }
                    } else if end == -1 {
                        let _start: usize = start as usize;
                        let mut ret = String::with_capacity(str_len - _start);
                        for c in string.chars().skip(_start) {
                            ret.push(c);
                        }
                        return ret;
                    } else {
                        let mut ret = String::with_capacity((end - start) as usize);
                        for (i, c) in string.char_indices().skip(start as usize) {
                            if i as i64 == end {
                                break;
                            }
                            ret.push(c);
                        }
                        return ret;
                    }
                }
                Index::Select(i) => {
                    let mut ret = String::with_capacity(1);
                    if i == -1 {
                        match string.chars().last() {
                            Some(c) => ret.push(c),
                            None => (),
                        }
                    } else {
                        match string.chars().nth(i as usize) {
                            Some(c) => ret.push(c),
                            None => (),
                        }
                    }
                    return ret;
                }
            }
        }

        async fn slice_array(&self, index: &Index, array: &Vec<&str>) -> String {
            let arr_len = array.len();
            match index {
                Index::Range(start, end) => {
                    if *start == -1 {
                        match array.get(arr_len) {
                            Some(s) => return s.to_string(),
                            None => return NULL.to_string(),
                        }
                    } else if *end == -1 {
                        match array.get(*start as usize..) {
                            Some(s) => s.join(" "),
                            None => NULL.to_string(),
                        }
                    } else {
                        let mut ret = String::new();
                        for i in *start..=*end {
                            match array.get(i as usize) {
                                Some(s) => ret.push_str(s),
                                None => {
                                    if i == *start {
                                        ret.push_str(NULL);
                                        break;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        return ret;
                    }
                }
                Index::Select(i) => {
                    if *i == -1 {
                        match array.get(arr_len - 1) {
                            Some(s) => s.to_string(),
                            None => NULL.to_string(),
                        }
                    } else {
                        match array.get(*i as usize) {
                            Some(s) => s.to_string(),
                            None => NULL.to_string(),
                        }
                    }
                }
            }
        }

        pub async fn format(&self, fmt_str: &'k str) -> String {
            //println!("{}", fmt_str);
            let mut ret = String::from(fmt_str);
            let indexes = self.get_indexes(fmt_str).await;
            for (start, end) in indexes {
                let substr_all: &str = fmt_str.get(start..=end).unwrap();
                //let substr_full = fmt_str.get(start..=end).unwrap();
                let substr = fmt_str.get(start + 1..=end - 1).unwrap();
                if !substr.contains(ARR) {
                    println!("no arr: {substr}");
                    match self.get(substr).await {
                        Some(v) => match v {
                            Value::String(ref s) => {
                                ret = ret.replace(substr_all, s);
                            }
                            Value::Array(ref a) => {
                                let repl: String = a.join(" ");
                                ret = ret.replace(substr_all, repl.as_str());
                            }
                            /*Value::Unsigned(ref n) => {
                                let _n = n.to_string();
                                ret.replace_range(start..=end, _n.as_str())
                            }*/
                            Value::Signed(n) => {
                                let _n = n.to_string();
                                ret = ret.replace(substr_all, _n.as_str());
                            }
                        },
                        None => panic!("invalid key: {}", substr),
                    }
                } else {
                    let mut key = self.parse_key(substr).await;
                    let val = match self.get(key.substr).await {
                        Some(v) => v,
                        None => panic!("invalid key: {}", key.substr),
                    };
                    if !key.index.is_empty() {
                        let mut repl = String::new();
                        let index_len = key.index.len();
                        for (i, index) in key.index.iter_mut().enumerate() {
                            match val {
                                Value::String(s) => {
                                    repl.push_str(self.slice_string(index, s).await.as_str());
                                }
                                Value::Array(ref a) => {
                                    repl.push_str(self.slice_array(index, a).await.as_str());
                                }
                                Value::Signed(n) => repl.push_str(n.to_string().as_str()),
                                /*Value::Unsigned(n) => repl.push_str(n.to_string().as_str()),*/
                            }
                            if i < index_len - 1 {
                                repl.push(' ');
                            }
                        }
                        ret = ret.replace(substr_all, repl.as_str());
                    }
                }
            }
            return ret;
        }

        //async fn get_range(&self, field: &'k str, )

        pub async fn get(&self, field: &'k str) -> Option<Value> {
            match field {
                "platform" => Some(Value::String(self.platform)),
                "id" => Some(Value::Signed(self.id)),
                "tags" => Some(Value::String(self.tags)),
                "source" => Some(Value::String(self.source)),
                "md5" => Some(Value::String(self.md5)),
                "file_size" => Some(Value::Signed(self.file_size)),
                "file_ext" => Some(Value::String(self.file_ext)),
                "rating" => Some(Value::String(self.rating)),
                "path" => Some(Value::String(self.path)),
                "general" => Some(Value::Array(self.general.clone())),
                "character" => Some(Value::Array(self.character.clone())),
                "copyright" => Some(Value::Array(self.copyright.clone())),
                "artist" => Some(Value::Array(self.artist.clone())),
                "metadata" => Some(Value::Array(self.metadata.clone())),
                _ => None,
            }
        }
    }
}

pub mod downloader {
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
        Client,
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

    /*pub const BLOCKSIZE: usize = 1048576;
    pub const GREEN: &str = "\x1b[32;1;1m";
    pub const RESET: &str = "\x1b[0m";*/

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

    pub async fn download<'download>(
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
                    let mut reader =
                        StreamReader::new(stream.map_err(|e| std::io::Error::other(e)));
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
}
