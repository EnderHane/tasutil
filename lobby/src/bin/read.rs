use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    path::PathBuf,
    str::FromStr,
};

use cgmath::MetricSpace;
use clap::Parser;
use itertools::Itertools;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version)]
struct Cli {
    path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    let path = cli.path.unwrap_or(env::current_dir().expect("Invalid cwd"));
    let edges: Vec<_> = WalkDir::new(path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            e.file_name()
                .to_string_lossy()
                .strip_suffix(".tas")
                .map(|n| n.split('-').map(ToOwned::to_owned).next_tuple::<(_, _)>())
                .flatten()
                .map(|val| {
                    (
                        val,
                        fs::read_to_string(e.path()).expect(&format!("Fail to read {e:?}")),
                    )
                })
        })
        .collect();
    let warp_coords: BTreeMap<_, _> = edges
        .iter()
        .filter(|((src, _), _)| !src.is_empty() && src.chars().all(|c| c.is_ascii_alphabetic()))
        .map(|((src, dst), content)| {
            let coord = content
                .lines()
                .map(str::trim_start)
                .filter_map(|ln| ln.strip_prefix("console"))
                .find_map(|r| r.find("load").map(|i| r.split(&r[..i])))
                .expect(&format!("Fail to parse load command in {src}-{dst}"))
                .filter_map(|s| f32::from_str(s).ok())
                .tuples::<(_, _)>()
                .nth(0)
                .expect(&format!("Fail to parse load command in {src}-{dst}"));
            (src, coord)
        })
        .collect();
    let graph: HashMap<_, HashMap<_, _>> = edges
        .iter()
        .map(|((src, dst), content)| {
            let time: usize = content
                .lines()
                .filter(|s| s.contains("Time:"))
                .find_map(|s| s.split(['(', ')']).nth(1))
                .map(usize::from_str)
                .expect(&format!("Fail to parse time in {src}-{dst}"))
                .expect(&format!("Fail to parse time in {src}-{dst}"));
            let activated_warps: Vec<_> = content
                .lines()
                .find_map(|s| s.trim_start().strip_prefix("ActiveWarps:"))
                .map(|s| s.trim().split(['(', ')', ',', ' ']))
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .map(f32::from_str)
                .filter_map(Result::ok)
                .tuples()
                .filter_map(|(x, y)| {
                    warp_coords
                        .iter()
                        .find(|(_, &(x1, y1))| {
                            cgmath::vec2(x, y).distance2(cgmath::vec2(x1, y1)) < 256.0
                        })
                        .map(|(&w, _)| w)
                })
                .collect();
            (src.as_str(), (dst.as_str(), (time, activated_warps)))
        })
        .into_grouping_map()
        .collect();

    let s = serde_json::to_string(&graph).unwrap();
    println!("{s}");
    if graph.is_empty() {
        eprintln!("Warning: no lobby files");
    }
}
