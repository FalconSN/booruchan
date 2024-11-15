use serde_json::{Map, Value};

pub async fn get_keys<'post, K>(
    post: &'post Map<String, Value>,
    keys: &Vec<K>,
    extra: Option<&Map<String, Value>>,
) -> Map<String, Value>
where
    K: AsRef<str>,
{
    let mut ret: Map<String, Value> = Map::new();
    let len: usize = keys.len();
    for key in keys {
        let key: &str = key.as_ref();
        if post.contains_key(key) {
            match post[key] {
                Value::String(ref s) => {
                    ret.insert(key.into(), s.as_str().into());
                }
                Value::Number(ref n) => {
                    ret.insert(key.into(), n.as_i64().unwrap().into());
                }
                Value::Array(ref a) => {
                    ret.insert(key.into(), {
                        let mut vals: Vec<String> = Vec::new();
                        a.iter().for_each(|x| match x {
                            Value::String(_s) => vals.push(_s.to_owned()),
                            Value::Number(_n) => vals.push(_n.to_string()),
                            Value::Bool(b) => vals.push({
                                if *b {
                                    "true".to_string()
                                } else {
                                    "false".to_string()
                                }
                            }),
                            _ => (),
                        });
                        vals.join(" ").into()
                    });
                }
                Value::Object(_) => println!(
                    "objects are not supported for inserting to database, skipping: {}",
                    key
                ),
                Value::Null => {
                    ret.insert(key.into(), Value::Null);
                }
                Value::Bool(b) => {
                    ret.insert(key.into(), b.into());
                }
            }
        }
    }
    match extra {
        Some(e) => keys.iter().map(|key| key.as_ref()).for_each(|key| {
            if e.contains_key(key) {
                match e[key] {
                    Value::String(ref s) => {
                        ret.insert(key.into(), s.as_str().into());
                    }
                    Value::Number(ref n) => {
                        ret.insert(key.into(), n.as_i64().unwrap().into());
                    }
                    _ => (),
                }
            }
        }),
        None => (),
    }
    assert_eq!(len, ret.len());
    return ret;
}

pub fn get_value<'a>(
    json_data: &'a Map<String, Value>,
    key: &str,
    parent: &str,
) -> Option<&'a Value> {
    if json_data.contains_key(parent) {
        if json_data[parent].as_object().unwrap().contains_key(key) {
            return Some(&json_data[parent][key]);
        }
    }
    if json_data.contains_key(key) {
        return Some(&json_data[key]);
    }
    return None;
}

pub fn get_value_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    let mut ret: Vec<String> = Vec::new();
    if map.contains_key(key) {
        match map[key] {
            Value::Array(ref array) => {
                for val in array {
                    match val {
                        Value::String(_) => ret.push(val.as_str().unwrap().to_owned()),
                        Value::Number(ref n) => ret.push(n.to_string()),
                        Value::Bool(b) => {
                            if *b {
                                ret.push("true".to_string());
                            } else {
                                ret.push("false".to_string());
                            }
                        }
                        Value::Null => ret.push("null".to_string()),
                        _ => (),
                    }
                }
            }
            Value::String(_) => ret.push(map[key].as_str().unwrap().to_owned()),
            Value::Number(ref n) => ret.push(n.to_string()),
            Value::Bool(b) => {
                if b {
                    ret.push("true".to_owned());
                } else {
                    ret.push("false".to_owned());
                }
            }
            Value::Null => ret.push("null".to_owned()),
            _ => (),
        }
    } else {
        return None;
    }
    return Some(ret.join(" "));
}
