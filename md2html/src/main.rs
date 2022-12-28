use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::{self, Cursor},
    path::{Path, PathBuf},
    time::Duration,
};

use log::{info, warn};

const MARKDOWN_PATH: &str = "markdown";
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

            let _ = files.insert(path, hash);
        });

    outdated_files
}

fn run() {
    info!("Watching files in {:?}", Path::new(MARKDOWN_PATH));
    let mut files = HashMap::new();
    loop {
        std::thread::sleep(POLL_DURATION);
        let outdated_files = update_files(&mut files);

        for file in outdated_files {
            match build(&file) {
                Ok(_) => info!("Re-compiled: {file:?}"),
                Err(_) => warn!("Failed to compile: {file:?}"),
            }
        }
    }
}

fn build(path: &Path) -> io::Result<()> {
    use pulldown_cmark::*;
    let path = path.to_path_buf();

    let markdown_input = fs::read_to_string(&path)?;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&markdown_input, options);

    // Write to a new String buffer.
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let mut name = path.file_name().unwrap().to_str().unwrap().to_string();
    name.pop();
    name.pop();
    name.push_str("html");

    let path = PathBuf::from(BUILD_PATH).join(name);

    fs::write(path, html_output)?;

    Ok(())
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

fn build_all() {
    info!("Compliling files in {:?}", Path::new(MARKDOWN_PATH));
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
        .for_each(|path| match build(&path) {
            Ok(_) => info!("Sucessfully compiled: {path:?}"),
            Err(_) => warn!("Failed to compile: {path:?}"),
        });
}

fn help() {
    println!(
        r#"Usage
   md2html [<command> <args>]

Options
   run           Watch for file changes and compile.
   build         Compile all markdown files"#
    );
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(arg) = args.get(0) {
        match arg.as_str() {
            "run" => run(),
            "build" => build_all(),
            _ => help(),
        }
    } else {
        help();
    }
}
