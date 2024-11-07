use crate::consts::NULL;
use serde_json::{Map, Value};

pub fn strip_last(str: &str, char: char) -> &str {
    match str.strip_suffix(char) {
        Some(s) => return s,
        None => return str,
    }
}

pub fn strip(str: &str, char: char) -> &str {
    match str.strip_prefix(char) {
        Some(s) => match s.strip_suffix(char) {
            Some(_s) => return _s,
            None => return s,
        },
        None => match str.strip_suffix(char) {
            Some(s) => return s,
            None => return str,
        },
    }
}

pub async fn format_multiple<'post>(
    post: &Map<String, Value>,
    tag_map: Option<&Map<String, Value>>,
    //extra: Option<&Vec<(&str, &str)>>,
    formats: Vec<&str>,
    restrict: Option<&Value>,
) -> Vec<String> {
    let mut ret: Vec<String> = Vec::new();
    for fmt in formats {
        ret.push(format(post, tag_map, fmt, restrict).await);
    }
    return ret;
}

pub async fn format<'post>(
    post: &Map<String, Value>,
    kw_map: Option<&Map<String, Value>>,
    fmt: &str,
    restrict: Option<&Value>,
) -> String {
    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut ret: String = String::from(fmt);
    let mut _start_index: usize = 0;
    let mut in_brackets: bool = false;
    for (index, char) in fmt.chars().enumerate() {
        if char == '{'
            && !{
                if index > 0 {
                    fmt.char_indices()
                        .nth(index - 1)
                        .is_some_and(|_char| _char.1 == '\\')
                } else {
                    false
                }
            }
        {
            in_brackets = true;
            _start_index = index;
        } else if char == '}'
            && !{
                if index > 0 {
                    fmt.char_indices()
                        .nth(index - 1)
                        .is_some_and(|_char| _char.1 == '\\')
                } else {
                    false
                }
            }
            && in_brackets
        {
            indexes.push((_start_index, index + 1));
            in_brackets = false;
        }
    }
    for (start, end) in indexes {
        let _substr_all: &str = fmt.get(start..end).unwrap();
        let mut _substr: &str = fmt.get(start + 1..end - 1).unwrap();
        let mut _replace: String = String::new();
        let _split: Vec<&str> = _substr
            .rsplitn(3, &['[', ']'])
            .filter(|s| !s.is_empty())
            .collect();
        let _split_len: usize = _split.len();
        if _split_len == 2 {
            _substr = _split[1];
        }
        if let Some(fmt_val) = match post.contains_key(_substr) {
            true => Some(&post[_substr]),
            false => match kw_map {
                Some(map) => match map.contains_key(_substr) {
                    true => Some(&map[_substr]),
                    false => None,
                },
                None => None,
            },
        } {
            let mut _repl: String = String::new();
            match fmt_val {
                Value::String(s) => _repl.push_str(s.as_str()),
                Value::Number(n) => _repl.push_str(n.to_string().as_str()),
                Value::Array(a) => {
                    if _split_len == 2 {
                        match a.get(_split[0].parse::<usize>().unwrap()) {
                            Some(v) => match v {
                                Value::String(s) => _repl.push_str(s.as_str()),
                                Value::Number(n) => _repl.push_str(n.to_string().as_str()),
                                Value::Null => _repl.push_str(NULL),
                                Value::Bool(b) => _repl.push_str(if *b { "true" } else { "false" }),
                                _ => {
                                    println!("nested objects and arrays can't be replaced with format string: {_substr_all}");
                                    _repl.push_str(NULL);
                                }
                            },
                            None => _repl.push_str(NULL),
                        }
                    } else {
                        let mut _temp: Vec<String> = Vec::new();
                        for val in a {
                            match val {
                                Value::String(s) => _temp.push(s.as_str().to_owned()),
                                Value::Number(n) => _temp.push(n.to_string()),
                                Value::Bool(b) => _temp.push(if *b {
                                    "true".to_owned()
                                } else {
                                    "false".to_owned()
                                }),
                                Value::Null => _temp.push(NULL.to_owned()),
                                Value::Array(_) | Value::Object(_) => {
                                    println!("nested objects and arrays can't be replaced with format string: {_substr_all}");
                                    _temp.push(NULL.to_owned());
                                }
                            }
                        }
                        _repl.push_str(_temp.join(" ").as_str());
                    }
                }
                Value::Bool(b) => _repl.push_str(if *b { "true" } else { "false" }),
                Value::Null => _repl.push_str(NULL),
                Value::Object(_) => {
                    println!("objects can't be replaced with format string: {_substr_all}");
                    _repl.push_str(NULL);
                }
            }
            ret = ret.replace(_substr_all, _repl.as_str());
        } else {
            ret = ret.replace(_substr_all, NULL);
        }
    }
    match restrict {
        Some(v) => {
            if v.is_array() {
                let as_arr = v.as_array().unwrap();
                as_arr.iter().for_each(|s| {
                    let _str: &str = s.as_str().unwrap();
                    ret = ret.replace(_str, "-");
                });
            } else if v.is_object() {
                let as_obj = v.as_object().unwrap();
                as_obj.iter().for_each(|(s, v)| {
                    ret = ret.replace(s, v.as_str().unwrap());
                })
            }
            /*
            let mut replace_indexes: Vec<usize> = Vec::new();
            for (pos, ch) in ret.char_indices() {
                for c in v.iter() {
                    if ch == *c {
                        replace_indexes.push(pos);
                    }
                }
            }
            for index in replace_indexes {
                ret.replace_range(index..index + 1, "-");
            }
            */
        }
        None => (),
    }
    return ret;
}
