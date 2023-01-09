mod post;
mod post_list;

use chrono::{DateTime, Datelike, Utc};
use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        print!("\x1b[94mINFO\x1b[0m ");
        println!($($arg)*);
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        print!("\x1b[93mWARN\x1b[0m '{}:{}:{}' ", file!(), line!(), column!());
        println!($($arg)*);
    }};
}

const MARKDOWN_PATH: &str = "markdown";
const TEMPLATE_PATH: &str = "templates";
const BUILD_PATH: &str = "build";
const POLL_DURATION: Duration = Duration::from_millis(500);

fn update_files(files: &mut HashMap<PathBuf, String>) -> Vec<PathBuf> {
    let mut outdated_files = Vec::new();

    walkdir::WalkDir::new(MARKDOWN_PATH)
        .into_iter()
        .flatten()
        .map(|dir_entry| dir_entry.path().to_path_buf())
        .filter(|path| {
            if let Some(ex) = path.extension() {
                ex.to_ascii_lowercase() == "md"
            } else {
                false
            }
        })
        .for_each(|path| {
            let hash = hash(&path).unwrap_or_default();
            if let Some(old_hash) = files.get(&path) {
                if &hash != old_hash {
                    outdated_files.push(path.clone());
                }
            }

            if files.insert(path.clone(), hash).is_none() {
                outdated_files.push(path);
            };
        });

    outdated_files
}

fn update_templates(markdown_files: &mut HashMap<PathBuf, String>) -> bool {
    let mut outdated_files = Vec::new();
    walkdir::WalkDir::new(TEMPLATE_PATH)
        .into_iter()
        .flatten()
        .map(|dir_entry| dir_entry.path().to_path_buf())
        .filter(|path| {
            if let Some(ex) = path.extension() {
                ex.to_ascii_lowercase() == "html"
            } else {
                false
            }
        })
        .for_each(|path| {
            let hash = hash(&path).unwrap_or_default();
            if let Some(old_hash) = markdown_files.get(&path) {
                if &hash != old_hash {
                    outdated_files.push(path.clone());
                }
            }

            if markdown_files.insert(path.clone(), hash).is_none() {
                outdated_files.push(path);
            };
        });

    !outdated_files.is_empty()
}

fn run() -> Result<(), Box<dyn Error>> {
    info!("Watching files in {:?}", Path::new(MARKDOWN_PATH));
    let mut files: HashMap<PathBuf, _> = HashMap::new();
    let mut markdown_files = HashMap::new();

    let mut list_template = fs::read_to_string("templates/post_list.html")?;
    let mut list_item_template = fs::read_to_string("templates/post_list_item.html")?;
    let mut post_template = fs::read_to_string("templates/post.html")?;

    //TODO: Log which files where changed and update less wastefully.
    loop {
        std::thread::sleep(POLL_DURATION);
        let outdated_files = update_files(&mut files);

        //Check if any templates are outdated.
        if update_templates(&mut markdown_files) {
            info!("Re-building templates.");

            //Update templates
            list_template = fs::read_to_string("templates/post_list.html")?;
            list_item_template = fs::read_to_string("templates/post_list_item.html")?;
            post_template = fs::read_to_string("templates/post.html")?;

            //Build post list
            let metadata: Vec<Metadata> = files
                .keys()
                .flat_map(|path| metadata(&fs::read_to_string(path)?, path))
                .collect();
            post_list::build(&list_template, &list_item_template, &metadata);

            //Build all posts
            for file in files.keys() {
                match post::build(&post_template, file) {
                    Ok(_) => info!("Compiled: {file:?}"),
                    Err(err) => warn!("Failed to compile: {file:?}\n{err}"),
                }
            }
        }
        //Update any outdated files.
        else if !outdated_files.is_empty() {
            //Build post list.
            let metadata: Vec<Metadata> = files
                .keys()
                .flat_map(|path| metadata(&fs::read_to_string(path)?, path))
                .collect();
            post_list::build(&list_template, &list_item_template, &metadata);

            //Build outdated posts.
            for file in outdated_files {
                match post::build(&post_template, &file) {
                    Ok(_) => info!("Compiled: {file:?}"),
                    Err(err) => warn!("Failed to compile: {file:?}\n{err}"),
                }
            }
        }
    }
}

//TODO: Swap to using posts?

#[derive(Default, Debug)]
pub struct Post {
    pub content: String,
    pub metadata: Metadata,
}

#[allow(unused)]
fn new_post(path: &Path) -> Post {
    todo!();
}

#[derive(Default, Debug)]
pub struct Metadata {
    pub title: String,
    pub summary: String,
    pub date: DateTime<Utc>,
    pub path: String,
    pub word_count: usize,
    pub read_time: f32,
}
impl Metadata {
    pub fn word_count(&self) -> String {
        if self.word_count != 1 {
            format!("{} words", self.word_count)
        } else {
            String::from("1 word")
        }
    }
    pub fn read_time(&self) -> String {
        if self.read_time < 1.0 {
            String::from("&lt;1 minute read")
        } else {
            format!("{} minute read", self.read_time as usize)
        }
    }
    pub fn date(&self) -> (String, String, i32) {
        let month = match self.date.month() {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => unreachable!(),
        };

        //Ordinal suffix.
        let day = self.date.day();
        let i = day % 10;
        let j = day % 100;
        let day = match i {
            1 if j != 11 => format!("{day}st"),
            2 if j != 12 => format!("{day}nd"),
            3 if j != 13 => format!("{day}rd"),
            _ => format!("{day}th"),
        };

        (day, month.to_string(), self.date.year())
    }
}

fn metadata(file: &str, path: &Path) -> Result<Metadata, Box<dyn Error>> {
    let end = file[2..].find("~~~").ok_or("Missing metadata")?;
    let config = &file[2..end];

    let mut metadata = Metadata::default();

    for line in config.split('\n') {
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim();
            match k {
                "title" => metadata.title = v.to_string(),
                "summary" => metadata.summary = v.to_string(),
                _ => continue,
            }
        }
    }

    metadata.date = fs::metadata(path)?.created()?.into();

    let mut pathbuf = path.to_path_buf();
    pathbuf.set_extension("html");
    metadata.path = pathbuf.file_name().unwrap().to_str().unwrap().to_string();

    metadata.word_count = count(file);
    metadata.read_time = metadata.word_count as f32 / 250.0;

    Ok(metadata)
}

pub fn count<S: AsRef<str>>(s: S) -> usize {
    let mut in_word = false;
    let mut consecutive_dashes = 0;

    let mut count = 0;

    for c in s.as_ref().chars() {
        if c.is_whitespace() {
            consecutive_dashes = 0;

            if in_word {
                count += 1;

                in_word = false;
            }
        } else {
            match c {
                '-' => {
                    consecutive_dashes += 1;

                    if consecutive_dashes > 1 && in_word {
                        if consecutive_dashes == 2 {
                            count += 1;
                        }

                        in_word = false;

                        continue;
                    }
                }
                _ => {
                    consecutive_dashes = 0;
                }
            }

            if !in_word {
                in_word = true;
            }
        }
    }

    if in_word {
        count += 1;
    }

    count
}

///Convert "markdown/example.md" to "build/example.html"
fn build_path(path: &Path) -> PathBuf {
    // let pathbuf = path.to_path_buf();
    // pathbuf.set_extension("html");

    let mut name = path.file_name().unwrap().to_str().unwrap().to_string();
    name.pop();
    name.pop();
    name.push_str("html");
    PathBuf::from(BUILD_PATH).join(name)
}

fn hash(path: impl AsRef<Path>) -> Result<String, Box<dyn Error>> {
    use blake3::*;

    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    let map = unsafe {
        memmap2::MmapOptions::new()
            .len(file_size as usize)
            .map(&file)?
    };

    let cursor = Cursor::new(map);
    let mut hasher = Hasher::new();
    hasher.update(cursor.get_ref());

    let mut output = hasher.finalize_xof();
    let mut block = [0; blake3::guts::BLOCK_LEN];
    let mut len = 32;
    let mut hex = String::new();

    while len > 0 {
        output.fill(&mut block);
        let hex_str = hex::encode(&block[..]);
        let take_bytes = std::cmp::min(len, block.len() as u64);
        hex.push_str(&hex_str[..2 * take_bytes as usize]);
        len -= take_bytes;
    }

    Ok(hex)
}

fn clean() {
    let markdown_files: Vec<PathBuf> = walkdir::WalkDir::new(MARKDOWN_PATH)
        .into_iter()
        .flatten()
        .map(|dir_entry| dir_entry.path().to_path_buf())
        .filter(|path| {
            if let Some(ex) = path.extension() {
                ex.to_ascii_lowercase() == "md"
            } else {
                false
            }
        })
        .collect();

    let build_files: Vec<PathBuf> = walkdir::WalkDir::new(BUILD_PATH)
        .into_iter()
        .flatten()
        .map(|dir_entry| dir_entry.path().to_path_buf())
        .filter(|path| path.is_file())
        .collect();

    let expected_files: Vec<PathBuf> = markdown_files
        .into_iter()
        .map(|file| {
            let mut name = file.file_name().unwrap().to_str().unwrap().to_string();
            name.pop();
            name.pop();
            name.push_str("html");
            PathBuf::from(BUILD_PATH).join(name)
        })
        .collect();

    for file in build_files {
        if !expected_files.contains(&file) {
            match fs::remove_file(&file) {
                Ok(_) => info!("Removed unexpected file: {file:?}"),
                Err(_) => warn!("Failed to remove unexpected file: {file:?}"),
            }
        }
    }
}

fn build_all() {
    todo!();
    // info!("Compliling files in {:?}", Path::new(MARKDOWN_PATH));
    // walkdir::WalkDir::new(MARKDOWN_PATH)
    //     .into_iter()
    //     .flatten()
    //     .map(|dir_entry| dir_entry.path().to_path_buf())
    //     .filter(|path| {
    //         if let Some(ex) = path.extension() {
    //             ex.to_ascii_lowercase() == "md"
    //         } else {
    //             false
    //         }
    //     })
    //     .for_each(|path| match post::build(&path) {
    //         Ok(_) => info!("Sucessfully compiled: {path:?}"),
    //         Err(_) => warn!("Failed to compile: {path:?}"),
    //     });
}

fn help() {
    println!(
        r#"Usage
   md2html [<command> <args>]

Options
   run           Watch for file changes and compile.
   build         Compile all markdown files.
   clean         Remove unused files."#
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(arg) = args.get(0) {
        match arg.as_str() {
            "build" => build_all(),
            "clean" => clean(),
            "run" => run().unwrap(),
            _ => help(),
        }
    } else {
        run().unwrap();
    }
}
