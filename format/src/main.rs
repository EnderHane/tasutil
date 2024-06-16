mod cli;
mod format;

use cli::Cli;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let cli = Cli::get();
    let path = cli
        .path
        .unwrap_or(env::current_dir().expect("Cannot access current directory!"));
    let mut buf = String::new();
    let mut count = 0u64;
    let mut mark_bp = false;
    WalkDir::new(path)
        .max_depth(if cli.recursive { usize::MAX } else { 2 })
        .into_iter()
        .filter_map(|e| {
            e.ok().filter(|e1| {
                e1.file_type().is_file()
                    && e1.path().extension().filter(|&ext| ext == "tas").is_some()
            })
        })
        .for_each(|e| {
            File::open(e.path())
                .expect(&format!("Cannot open file {:?}", e.path()))
                .read_to_string(&mut buf)
                .expect(&format!("Cannot read file {:?}", e.path()));
            let [bp, _] = criticize_format_issue(e.path(), &buf);
            mark_bp |= bp;
            buf.clear();
            count += 1;
        });
    println!("{count} files scanned");
    if !mark_bp {
        println!("No breakpoints found");
    }
}

fn criticize_format_issue(filepath: &Path, content: &str) -> [bool; 2] {
    let mut mark_bp = false;
    for (ln, content) in format::find_breakpoints(content) {
        if !mark_bp {
            println!("breakpoints found in {filepath:?}");
            mark_bp = true;
        }
        println!("line\t{ln}\t{content}");
    }
    let mut mark_sl = true;
    let starts: Vec<_> = format::find_start_labels(content).collect();
    match starts.len() {
        0 => println!("start label (#Start) not found in {filepath:?}"),
        1 => mark_sl = false,
        2.. => {
            println!("multiple start labels found in {filepath:?}");
            for (ln, content) in starts {
                println!("line\t{ln}\t{content}");
            }
        }
    }
    [mark_bp, mark_sl]
}
