use std::{fmt, process::exit};

use crate::{platforms::base::Platform, pub_struct, statics::HOME};
use serde::{
    de::{self, DeserializeSeed, Deserializer, MapAccess, Visitor},
    Deserialize,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Compress {
    pub target_dir: String,
    //pub subdir: Option<String>,
    pub filename: String,
    pub size: (u32, u32),
}

impl Default for Compress {
    fn default() -> Self {
        Self {
            target_dir: format!("{}/booruchan", HOME.as_str()),
            //subdir: Some("{platform}".to_string()),
            filename: "{id}.{file_ext}".to_string(),
            size: (5000, 5000),
        }
    }
}

//#[derive(Debug)]
#[allow(dead_code)]
pub struct Config {
    pub global: GlobalConfig,
    // platforms
    pub platforms: Vec<Platform>,
    /*pub konachan: Option<PlatformConfig>,
    pub sakugabooru: Option<PlatformConfig>,
    pub yandere: Option<PlatformConfig>,
    pub gelbooru: Option<PlatformConfig>,*/
}

impl Config {
    pub fn load() -> Self {
        use crate::statics::ARGS;
        use std::{fs, io::BufReader, io::ErrorKind, process::exit};

        let conf: Self;
        match fs::OpenOptions::new().read(true).open(&ARGS.config.path) {
            Ok(f) => {
                conf = serde_json::from_reader(BufReader::new(f)).unwrap();
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound | ErrorKind::PermissionDenied => {
                    if ARGS.config.is_custom {
                        eprintln!(
                            "config file is not found or accessible: {}",
                            ARGS.config.path.display()
                        );
                        exit(2);
                    }
                    conf = serde_json::from_str("{}").unwrap();
                }
                _ => panic!("{e:?}"),
            },
        }
        return conf;
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            ToCloud,
            Delete,
            Cloud,
            Database,
            TargetDir,
            //Subdir,
            Filename,
            Compress,
            Skip,
            Sleep,
            Retries,
            RetrySleep,
            Timeout,
            FilenameRepl,
            DirnameRepl,
            Tags,
            Blacklist,
            // platforms
            Yandere,
            Sakugabooru,
            Konachan,
            Gelbooru,
        }

        struct ConfigVisitor;
        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Config")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut global_config: GlobalConfig = GlobalConfig::default();
                let mut to_cloud: Option<()> = None;
                let mut delete: Option<()> = None;
                let mut cloud: Option<()> = None;
                let mut database: Option<()> = None;
                let mut target_dir: Option<()> = None;
                //let mut subdir: Option<()> = None;
                let mut filename: Option<()> = None;
                let mut compress: Option<()> = None;
                let mut skip: Option<()> = None;
                let mut sleep: Option<()> = None;
                let mut retries: Option<()> = None;
                let mut retry_sleep: Option<()> = None;
                let mut timeout: Option<()> = None;
                let mut filename_repl: Option<()> = None;
                let mut dirname_repl: Option<()> = None;
                let mut tags: Option<()> = None;
                let mut blacklist: Option<()> = None;
                let mut platforms: Vec<Platform> = Vec::new();
                let mut yandere = false;
                let mut sakugabooru = false;
                let mut konachan = false;
                //let mut gelbooru = false;
                /*let mut yandere: Option<PlatformConfig> = None;
                let mut sakugabooru: Option<PlatformConfig> = None;
                let mut konachan: Option<PlatformConfig> = None;
                let mut gelbooru: Option<PlatformConfig> = None;*/

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::ToCloud => {
                            if to_cloud.is_some() {
                                return Err(de::Error::duplicate_field("to_cloud"));
                            }
                            let val = map.next_value()?;
                            to_cloud = Some(());
                            global_config.to_cloud = val;
                        }
                        Field::Delete => {
                            if delete.is_some() {
                                return Err(de::Error::duplicate_field("delete"));
                            }
                            let val = map.next_value()?;
                            delete = Some(());
                            global_config.delete = val;
                        }
                        Field::Cloud => {
                            if cloud.is_some() {
                                return Err(de::Error::duplicate_field("cloud"));
                            }
                            let val = map.next_value()?;
                            cloud = Some(());
                            global_config.cloud = Some(val);
                        }
                        Field::Database => {
                            if database.is_some() {
                                return Err(de::Error::duplicate_field("database"));
                            }
                            let val = map.next_value()?;
                            database = Some(());
                            global_config.database = expand_home(val);
                        }
                        Field::TargetDir => {
                            if target_dir.is_some() {
                                return Err(de::Error::duplicate_field("base_dir"));
                            }
                            let val: String = map.next_value()?;
                            target_dir = Some(());
                            global_config.target_dir = expand_home(val);
                        }
                        /*Field::Subdir => {
                            if subdir.is_some() {
                                return Err(de::Error::duplicate_field("output_dir"));
                            }
                            let val = map.next_value()?;
                            subdir = Some(());
                            global_config.subdir = val;
                        }*/
                        Field::Filename => {
                            if filename.is_some() {
                                return Err(de::Error::duplicate_field("filename"));
                            }
                            let val = map.next_value()?;
                            filename = Some(());
                            global_config.filename = val;
                        }
                        Field::Compress => {
                            if compress.is_some() {
                                return Err(de::Error::duplicate_field("compress"));
                            }
                            let val: Option<Compress> = map.next_value()?;
                            match val {
                                Some(mut _compress) => {
                                    _compress.target_dir = expand_home(_compress.target_dir);
                                    global_config.compress = Some(_compress);
                                    compress = Some(())
                                }
                                None => (),
                            }
                        }
                        Field::Skip => {
                            if skip.is_some() {
                                return Err(de::Error::duplicate_field("skip"));
                            }
                            let val = map.next_value()?;
                            skip = Some(());
                            global_config.skip = val;
                        }
                        Field::Sleep => {
                            if sleep.is_some() {
                                return Err(de::Error::duplicate_field("sleep"));
                            }
                            let val = map.next_value()?;
                            sleep = Some(());
                            global_config.sleep = val;
                        }
                        Field::Retries => {
                            if retries.is_some() {
                                return Err(de::Error::duplicate_field("retries"));
                            }
                            let val = map.next_value()?;
                            retries = Some(());
                            global_config.retries = val;
                        }
                        Field::RetrySleep => {
                            if retry_sleep.is_some() {
                                return Err(de::Error::duplicate_field("retry_sleep"));
                            }
                            let val = map.next_value()?;
                            retry_sleep = Some(());
                            global_config.retry_sleep = val;
                        }
                        Field::Timeout => {
                            if timeout.is_some() {
                                return Err(de::Error::duplicate_field("timeout"));
                            }
                            let val = map.next_value()?;
                            timeout = Some(());
                            global_config.timeout = val;
                        }
                        Field::FilenameRepl => {
                            if filename_repl.is_some() {
                                return Err(de::Error::duplicate_field("filename_repl"));
                            }
                            let val = map.next_value()?;
                            filename_repl = Some(());
                            global_config.filename_repl = val;
                        }
                        Field::DirnameRepl => {
                            if dirname_repl.is_some() {
                                return Err(de::Error::duplicate_field("dirname_repl"));
                            }
                            let val = map.next_value()?;
                            dirname_repl = Some(());
                            global_config.dirname_repl = val;
                        }
                        Field::Tags => {
                            if tags.is_some() {
                                return Err(de::Error::duplicate_field("tags"));
                            }
                            let val = map.next_value()?;
                            tags = Some(());
                            global_config.tags = val;
                        }
                        Field::Blacklist => {
                            if blacklist.is_some() {
                                return Err(de::Error::duplicate_field("blacklist"));
                            }
                            let val = map.next_value()?;
                            blacklist = Some(());
                            global_config.blacklist = val;
                        }
                        Field::Yandere => {
                            if yandere {
                                return Err(de::Error::duplicate_field("yandere"));
                            }
                            platforms.push(Platform::Yandere(map.next_value_seed(&global_config)?));
                            yandere = true;
                            /*if yandere.is_some() {
                                return Err(de::Error::duplicate_field("yandere"));
                            }
                            let val = map.next_value_seed(&global_config)?;
                            yandere = Some(val);*/
                        }
                        Field::Sakugabooru => {
                            if sakugabooru {
                                return Err(de::Error::duplicate_field("sakugabooru"));
                            }
                            platforms
                                .push(Platform::Sakugabooru(map.next_value_seed(&global_config)?));
                            sakugabooru = true;

                            /*if sakugabooru.is_some() {
                                return Err(de::Error::duplicate_field("sakugabooru"));
                            }
                            let val = map.next_value_seed(&global_config)?;
                            sakugabooru = Some(val);*/
                        }
                        Field::Konachan => {
                            if konachan {
                                return Err(de::Error::duplicate_field("konachan"));
                            }
                            platforms
                                .push(Platform::Konachan(map.next_value_seed(&global_config)?));
                            konachan = true;

                            /*if konachan.is_some() {
                                return Err(de::Error::duplicate_field("konachan"));
                            }
                            let val = map.next_value_seed(&global_config)?;
                            konachan = Some(val);*/
                        }
                        Field::Gelbooru => {
                            let _: PlatformConfig = map.next_value_seed(&global_config)?;
                        } /*Field::Gelbooru => {
                              if gelbooru {
                                  return Err(de::Error::duplicate_field("gelbooru"));
                              }
                              platforms
                                  .push(Platform::Gelbooru(map.next_value_seed(&global_config)?));
                              gelbooru = true;
                              /*if gelbooru.is_some() {
                                  return Err(de::Error::duplicate_field("gelbooru"));
                              }
                              let val = map.next_value_seed(&global_config)?;
                              gelbooru = Some(val);*/
                          }*/
                    }
                }
                Ok(Config {
                    global: global_config,
                    platforms,
                    /*yandere,
                    sakugabooru,
                    konachan,
                    gelbooru,*/
                })
            }
        }
        const FIELDS: &[&str] = &[
            "to_cloud",
            "delete",
            "cloud",
            "database",
            "target_dir",
            //"subdir",
            "filename",
            "compress",
            /*"compress_base",
            "compress_subdir",
            "compress_filename",
            "compress_size",*/
            "skip",
            "sleep",
            "retries",
            "retry_sleep",
            "timeout",
            "filename_repl",
            "dirname_repl",
            "tags",
            "blacklist",
            "yandere",
            "sakugabooru",
            "konachan",
            //"gelbooru",
        ];
        deserializer.deserialize_struct("Config", FIELDS, ConfigVisitor)
    }
}

//#[derive(Debug)]
pub_struct!(GlobalConfig {
    to_cloud: bool,
    delete: bool,
    cloud: Option<String>,
    database: String,
    target_dir: String,
    //subdir: Option<String>,
    filename: String,
    compress: Option<Compress>,
    skip: bool,
    sleep: f32,
    retries: i64,
    retry_sleep: f32,
    timeout: f32,
    filename_repl: Vec<String>,
    dirname_repl: Vec<String>,
    tags: Vec<String>,
    blacklist: Vec<String>,
});

#[allow(unused_assignments)]
impl<'de, 'a> DeserializeSeed<'de> for &'a GlobalConfig {
    type Value = PlatformConfig;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            ToCloud,
            Delete,
            Cloud,
            Database,
            TargetDir,
            //Subdir,
            Filename,
            Compress,
            Skip,
            Sleep,
            Retries,
            RetrySleep,
            Timeout,
            FilenameRepl,
            DirnameRepl,
            Tags,
            Blacklist,
            ApiKey,
            UserId,
        }

        struct PlatformConfigVisitor<'a>(&'a GlobalConfig);

        impl<'de, 'a> Visitor<'de> for PlatformConfigVisitor<'a> {
            type Value = PlatformConfig;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PlatformConfig")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut to_cloud: Option<bool> = None;
                let mut delete: Option<bool> = None;
                let mut cloud: Option<String> = None;
                let mut database: Option<String> = None;
                let mut target_dir: Option<String> = None;
                //let mut subdir: Option<String> = None;
                let mut filename: Option<String> = None;
                let mut compress: Option<Compress> = None;
                let mut skip: Option<bool> = None;
                let mut sleep: Option<f32> = None;
                let mut retries: Option<i64> = None;
                let mut retry_sleep: Option<f32> = None;
                let mut timeout: Option<f32> = None;
                let mut filename_repl: Option<Vec<String>> = None;
                let mut dirname_repl: Option<Vec<String>> = None;
                let mut tags: Option<Vec<String>> = None;
                let mut blacklist: Option<Vec<String>> = None;
                let mut api_key: Option<String> = None;
                let mut user_id: Option<u64> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::ToCloud => {
                            if to_cloud.is_some() {
                                return Err(de::Error::duplicate_field("to_cloud"));
                            }
                            let val = map.next_value()?;
                            to_cloud = Some(val);
                        }
                        Field::Delete => {
                            if delete.is_some() {
                                return Err(de::Error::duplicate_field("delete"));
                            }
                            let val = map.next_value()?;
                            delete = Some(val);
                        }
                        Field::Cloud => {
                            if cloud.is_some() {
                                return Err(de::Error::duplicate_field("cloud"));
                            }
                            let val = map.next_value()?;
                            cloud = Some(val);
                        }
                        Field::Database => {
                            if database.is_some() {
                                return Err(de::Error::duplicate_field("database"));
                            }
                            let val = map.next_value()?;
                            database = Some(val);
                        }
                        Field::TargetDir => {
                            if target_dir.is_some() {
                                return Err(de::Error::duplicate_field("base_dir"));
                            }
                            let val: String = map.next_value()?;
                            target_dir = Some(expand_home(val));
                        }
                        /*Field::Subdir => {
                            if subdir.is_some() {
                                return Err(de::Error::duplicate_field("output_dir"));
                            }
                            let val = map.next_value()?;
                            subdir = Some(val);
                        }*/
                        Field::Filename => {
                            if filename.is_some() {
                                return Err(de::Error::duplicate_field("filename"));
                            }
                            let val = map.next_value()?;
                            filename = Some(val);
                        }
                        Field::Compress => {
                            if compress.is_some() {
                                return Err(de::Error::duplicate_field("compress"));
                            }
                            let val: Option<Compress> = map.next_value()?;
                            match val {
                                Some(mut _compress) => {
                                    _compress.target_dir = expand_home(_compress.target_dir);
                                    compress = Some(_compress);
                                }
                                None => (),
                            }
                        }
                        Field::Skip => {
                            if skip.is_some() {
                                return Err(de::Error::duplicate_field("skip"));
                            }
                            let val = map.next_value()?;
                            skip = Some(val);
                        }
                        Field::Sleep => {
                            if sleep.is_some() {
                                return Err(de::Error::duplicate_field("sleep"));
                            }
                            let val = map.next_value()?;
                            sleep = Some(val);
                        }
                        Field::Retries => {
                            if retries.is_some() {
                                return Err(de::Error::duplicate_field("retries"));
                            }
                            let val = map.next_value()?;
                            retries = Some(val);
                        }
                        Field::RetrySleep => {
                            if retry_sleep.is_some() {
                                return Err(de::Error::duplicate_field("retry_sleep"));
                            }
                            let val = map.next_value()?;
                            retry_sleep = Some(val);
                        }
                        Field::Timeout => {
                            if timeout.is_some() {
                                return Err(de::Error::duplicate_field("timeout"));
                            }
                            let val = map.next_value()?;
                            timeout = Some(val);
                        }
                        Field::FilenameRepl => {
                            if filename_repl.is_some() {
                                return Err(de::Error::duplicate_field("filename_repl"));
                            }
                            let val = map.next_value()?;
                            filename_repl = Some(val);
                        }
                        Field::DirnameRepl => {
                            if dirname_repl.is_some() {
                                return Err(de::Error::duplicate_field("dirname_repl"));
                            }
                            let val = map.next_value()?;
                            dirname_repl = Some(val);
                        }
                        Field::Tags => {
                            if tags.is_some() {
                                return Err(de::Error::duplicate_field("tags"));
                            }
                            let val = map.next_value()?;
                            tags = Some(val);
                        }
                        Field::Blacklist => {
                            if blacklist.is_some() {
                                return Err(de::Error::duplicate_field("blacklist"));
                            }
                            let val = map.next_value()?;
                            blacklist = Some(val);
                        }
                        Field::ApiKey => {
                            if api_key.is_some() {
                                return Err(de::Error::duplicate_field("api_key"));
                            }
                            let val = map.next_value()?;
                            api_key = Some(val);
                        }
                        Field::UserId => {
                            if user_id.is_some() {
                                return Err(de::Error::duplicate_field("user_id"));
                            }
                            let val = map.next_value()?;
                            user_id = Some(val);
                        }
                    }
                }
                Ok(PlatformConfig {
                    to_cloud: to_cloud.unwrap_or(self.0.to_cloud),
                    delete: delete.unwrap_or(self.0.delete.clone()),
                    cloud: match to_cloud {
                        Some(b) => match b {
                            true => match cloud {
                                Some(cloud) => Some(cloud),
                                None => match self.0.cloud {
                                    Some(ref cloud) => Some(cloud.to_owned()),
                                    None => {
                                        eprintln!("local config to_cloud is true but couldn't find cloud.");
                                        exit(2);
                                    }
                                },
                            },
                            false => None,
                        },
                        None => match self.0.to_cloud {
                            true => match cloud {
                                Some(cloud) => Some(cloud),
                                None => match self.0.cloud {
                                    Some(ref cloud) => Some(cloud.to_owned()),
                                    None => {
                                        eprintln!("global config to_cloud is true but couldn't find cloud");
                                        exit(2);
                                    }
                                },
                            },
                            false => None,
                        },
                    },
                    database: database.unwrap_or(self.0.database.clone()),
                    target_dir: target_dir.unwrap_or(self.0.target_dir.clone()),
                    /*subdir: if subdir.is_some() {
                        subdir
                    } else if self.0.subdir.is_some() {
                        self.0.subdir.clone()
                    } else {
                        None
                    },*/
                    filename: filename.unwrap_or(self.0.filename.clone()),
                    compress: match compress {
                        Some(c) => Some(c),
                        None => self.0.compress.clone(),
                    },
                    skip: skip.unwrap_or(self.0.skip.clone()),
                    sleep: sleep.unwrap_or(self.0.sleep.clone()),
                    retries: retries.unwrap_or(self.0.retries.clone()),
                    retry_sleep: retry_sleep.unwrap_or(self.0.retry_sleep.clone()),
                    timeout: timeout.unwrap_or(self.0.timeout.clone()),
                    filename_repl: filename_repl.unwrap_or(self.0.filename_repl.clone()),
                    dirname_repl: dirname_repl.unwrap_or(self.0.dirname_repl.clone()),
                    tags: tags.unwrap_or(self.0.tags.clone()),
                    blacklist: blacklist.unwrap_or(self.0.blacklist.clone()),
                    api_key,
                    user_id,
                })
            }
        }
        const FIELDS: &[&str] = &[
            "to_cloud",
            "delete",
            "cloud",
            "database",
            "target_dir",
            //"subdir",
            "filename",
            "compress",
            "skip",
            "sleep",
            "retries",
            "retry_sleep",
            "timeout",
            "filename_repl",
            "dirname_repl",
            "tags",
            "api_key",
            "user_id",
        ];
        deserializer.deserialize_struct("PlatformConfig", FIELDS, PlatformConfigVisitor(self))
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            to_cloud: false,
            delete: false,
            cloud: None,
            database: format!("{}/.archives/booruchan.db", HOME.as_str()),
            target_dir: format!("{}/booruchan/{{platform}}", HOME.as_str()),
            //subdir: None,
            filename: "{id}.{file_ext}".into(),
            compress: None,
            skip: true,
            sleep: 1.5,
            retries: 5,
            retry_sleep: 1.0,
            timeout: 30.0,
            filename_repl: [":", "!", "?", "*", "\"", "'", "/"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            dirname_repl: [":", "!", "?", "*", "\"", "'"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            tags: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

pub_struct!(PlatformConfig {
    to_cloud: bool,
    delete: bool,
    cloud: Option<String>,
    database: String,
    target_dir: String,
    //subdir: Option<String>,
    filename: String,
    compress: Option<Compress>,
    skip: bool,
    sleep: f32,
    retries: i64,
    retry_sleep: f32,
    timeout: f32,
    filename_repl: Vec<String>,
    dirname_repl: Vec<String>,
    tags: Vec<String>,
    blacklist: Vec<String>,
    api_key: Option<String>,
    user_id: Option<u64>,
});

fn expand_home(string: String) -> String {
    match string.starts_with("~/") {
        true => format!("{}/{}", HOME.as_str(), string.trim_start_matches("~/")),
        false => string,
    }
}
