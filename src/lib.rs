mod args;
mod downloader;
pub use downloader::Downloader;
mod config;
pub mod platforms;
pub mod rclone;
pub use config::{Config, PlatformConfig};
//pub use platforms::base::init_platforms;
pub mod utils;
pub mod worker;
pub use args::Args;

#[macro_export]
macro_rules! pub_struct {
    ($name:ident {$($field:ident: $t:ty,)*}) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $t),*
        }
    }
}

pub mod consts {
    pub const NULL: &str = "null";
    pub const BLOCKSIZE: usize = 1048576;
    pub const GREEN: &str = "\x1b[32;1;1m";
    pub const RESET: &str = "\x1b[0m";

    //pub const DUPLICATE: &str = "_duplicate_";
}

pub mod statics {
    use std::env::var;
    use std::sync::LazyLock;

    use crate::Args;

    pub static ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());
    //pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::load());
    pub static HOME: LazyLock<String> = LazyLock::new(|| match var("HOME") {
        Ok(v) => v.trim_end_matches('/').to_string(),
        Err(_) => {
            eprintln!("HOME variable is not set, using current directory as fallback.");
            ".".into()
        }
    });
}

pub(crate) mod fmt {
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
    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct Keywords<'k> {
        pub platform: &'k str,
        pub id: i64,
        pub tags: &'k str,
        pub source: &'k str,
        pub md5: &'k str,
        pub file_size: i64,
        pub file_ext: &'k str,
        pub rating: &'k str,
        //pub path: &'k str,
        pub general: Vec<&'k str>,
        pub character: Vec<&'k str>,
        pub copyright: Vec<&'k str>,
        pub artist: Vec<&'k str>,
        pub metadata: Vec<&'k str>,
        pub circle: Vec<&'k str>,
        pub faults: Vec<&'k str>,
        pub style: Vec<&'k str>,
    }
    impl<'k> Keywords<'k> {
        fn parse_key(&self, substr: &'k str) -> Key {
            let mut key = Key {
                substr,
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
                        if index.len() == 1 {
                            // index is [:], we'll consider it as it as [0:-1] which means from start to end
                            // but should we when there are possibly more indexes separated with ','?
                            key.index.push(Index::Range(0, -1));
                            continue;
                        }
                        // not sure if this is supposed to return 1 or 2 items
                        let end = index.rsplitn(2, RANGE_SEP).collect::<Vec<&str>>();
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
                            panic!(
                                "error while parsing: start: {start:?}, key: {:?}",
                                key.substr
                            );
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
        fn get_indexes(&self, fmt_str: &str) -> Vec<(usize, usize)> {
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

        fn slice_string(&self, index: &Index, string: &str) -> String {
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

        fn slice_array(&self, index: &Index, array: &Vec<&str>) -> String {
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
                        let mut ret: Vec<&str> = Vec::new();
                        //let mut ret = String::new();
                        for i in *start..=*end {
                            match array.get(i as usize) {
                                Some(&s) => ret.push(s),
                                None => {
                                    if i == *start {
                                        ret.push(NULL);
                                        break;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        return ret.join(" ");
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

        pub fn format(&self, fmt_str: &'k str) -> String {
            //println!("{}", fmt_str);
            let mut ret = String::from(fmt_str);
            let indexes = self.get_indexes(fmt_str);
            for (start, end) in indexes {
                let substr_all: &str = fmt_str.get(start..=end).unwrap();
                //let substr_full = fmt_str.get(start..=end).unwrap();
                let substr = fmt_str.get(start + 1..=end - 1).unwrap();
                if !substr.contains(ARR) {
                    match self.get(substr) {
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
                    let mut key = self.parse_key(substr);
                    let val = match self.get(key.substr) {
                        Some(v) => v,
                        None => panic!("invalid key: {}", key.substr),
                    };
                    if !key.index.is_empty() {
                        let mut repl = String::new();
                        let index_len = key.index.len();
                        for (i, index) in key.index.iter_mut().enumerate() {
                            match val {
                                Value::String(s) => {
                                    repl.push_str(self.slice_string(index, s).as_str());
                                }
                                Value::Array(ref a) => {
                                    repl.push_str(self.slice_array(index, a).as_str());
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

        pub fn get(&self, field: &'k str) -> Option<Value> {
            match field {
                "platform" => Some(Value::String(self.platform)),
                "id" => Some(Value::Signed(self.id)),
                "tags" => Some(Value::String(self.tags)),
                "source" => Some(Value::String(self.source)),
                "md5" => Some(Value::String(self.md5)),
                "file_size" => Some(Value::Signed(self.file_size)),
                "file_ext" => Some(Value::String(self.file_ext)),
                "rating" => Some(Value::String(self.rating)),
                //"path" => Some(Value::String(self.path)),
                "general" => Some(Value::Array(self.general.clone())),
                "character" => Some(Value::Array(self.character.clone())),
                "copyright" => Some(Value::Array(self.copyright.clone())),
                "artist" => Some(Value::Array(self.artist.clone())),
                "metadata" => Some(Value::Array(self.metadata.clone())),
                "style" => Some(Value::Array(self.style.clone())),
                "faults" => Some(Value::Array(self.faults.clone())),
                _ => None,
            }
        }
    }
}
