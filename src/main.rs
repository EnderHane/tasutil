use std::{env, fs};
use std::collections::{HashSet, BTreeSet, BTreeMap};
use std::ops::Not;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::cli::{Cli, Command, FormatCommand, LobbyCommand};

mod cli;
mod format;
mod lobby;

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Format { command } => _format(command),
        Command::Lobby { command } => _lobby(command),
    }
}


fn _format(command: FormatCommand) {
    match command {
        FormatCommand::ScanBreakpoint => _scan_breakpoint()
    }
}


fn _scan_breakpoint() {
    let (file_num, bp_num) = format::scan_breakpoint(env::current_dir().unwrap()).iter()
        .fold((0, 0), |(file_num, bp_num), (s, r)|
            (file_num + 1, match r {
                Ok(map) => {
                    if !map.is_empty() {
                        let p = Path::new(s).strip_prefix(env::current_dir().unwrap()).unwrap();
                        println!("{}", p.display());
                        map.iter().for_each(|(n, l)| println!("{n}\t|{l}"));
                        println!();
                    }
                    bp_num + map.len()
                }
                Err(_) => {
                    println!("Error occurred reading {s}");
                    bp_num
                }
            }));
    println!("Found {bp_num} breakpoints in total {file_num} TAS files")
}


fn _lobby(command: LobbyCommand) {
    match command {
        LobbyCommand::Info { dir } => _info(dir),
        LobbyCommand::Route { dir, num, show_arc } => _route(dir, num, show_arc),
        LobbyCommand::GenerateInput { string, csv, lobby_dir } => _generate_input(string, csv, lobby_dir)
    }
}


fn _info(dir: Option<PathBuf>) {
    let path = dir.unwrap_or(env::current_dir().unwrap());
    let (succ, fail) = lobby::lobby_map(path);
    if succ.is_empty().not() {
        println!("Arc\tTime\tFile");
        succ.iter()
            .for_each(|((a, b), (w, p))| println!("{a}->{b}\t{w}\t{p}"))
    }
    println!();
    if succ.len() >= 2 {
        let indices = succ.iter().flat_map(|((a, b), _)| [*a, *b]).collect::<HashSet<_>>();
        let (first, last) = (*indices.iter().min().unwrap(), *indices.iter().max().unwrap());
        println!("Matrix in CSV:");
        for i in first..=last {
            for j in first..=last {
                if let Some((w, _)) = succ.get(&(i, j)) {
                    print!("{w}")
                }
                if j != last {
                    print!(",")
                }
            }
            println!()
        }
    }
    println!();
    if fail.is_empty().not() {
        println!("Warning, NO timestamp found in these files:");
        fail.iter().for_each(|(_, p)| println!("{p}"))
    }
    println!();
}


fn _route(dir: Option<PathBuf>, num: Option<u32>, show_arc: bool) {
    let path = dir.unwrap_or(env::current_dir().unwrap());
    let (succ, fail) = lobby::lobby_map(&path);
    if fail.is_empty() {
        if succ.is_empty().not() {
            let indices = succ.iter().flat_map(|((a, b), _)| [*a, *b]).collect::<BTreeSet<_>>();
            let (first, last) = (*indices.iter().min().unwrap(), *indices.iter().max().unwrap());
            let buffer_size = num.unwrap_or(1);
            let lobby = succ.iter().map(|((a, b), (w, _))| ((*a, *b), *w)).collect();
            let (path_count, results) = lobby::route(&lobby, first, last, buffer_size);
            println!("Found {path_count} paths in {path:?}");
            println!("Best {buffer_size} paths are");
            for (i, (len, p)) in results.iter().enumerate() {
                let path_string = p.iter()
                    .fold::<(String, Option<&u32>), _>((String::new(), None),
                        |(mut acc, pre), vert| {
                            if let Some(vert_pre) = pre {
                                if show_arc {
                                    if let Some(w) = lobby.get(&(*vert_pre, *vert)) {
                                        acc.push_str("-<");
                                        acc.push_str(w.to_string().as_str());
                                        acc.push_str(">-");
                                    }
                                } else {
                                    acc.push('-');
                                }
                            }
                            acc.push_str(vert.to_string().as_str());
                            (acc, Some(vert))
                        }).0;
                println!("{})\t[{}]\t{}", i + 1, len, path_string);
            }
        }
    } else {
        println!("Warning, NO timestamp found in these files, and the algorithm will NOT be run:");
        fail.iter().for_each(|(_, p)| println!("{p}"))
    }
}

fn _generate_input(string: String, csv: PathBuf, lobby_dir: PathBuf) {
    let (succ, fail) = lobby::lobby_map(lobby_dir);
    let lobby = succ.into_iter()
        .map(|(ab, (w, p))| (ab, (Some(w), p)))
        .chain(
            fail.into_iter()
                .map(|(ab, p)| (ab, (None, p)))
        )
        .collect::<BTreeMap<_, _>>();
    let csv_content = fs::read_to_string(&csv).unwrap_or_else(|_| panic!("Cannot read lobby CSV {}", csv.display()));
    let table = csv_content.lines()
        .filter_map(|l| {
            let pair = l.split(',').collect::<Vec<_>>();
            pair.len().ge(&2).then_some((pair[0].parse().unwrap(), pair[1]))
        })
        .collect::<BTreeMap<u32, _>>();
    let arcs = string.split('-')
        .map(|s| s.parse::<u32>().unwrap())
        .fold((vec![], None), |(mut acc, pre), vert| {
            match pre {
                Some(vert_pre) => {
                    acc.push((vert_pre, vert));
                    (acc, Some(vert))
                }
                None => (acc, Some(vert))
            }
        }).0;
    arcs.iter().for_each(|(a, b)| {
        if let Some((_, f_path)) = lobby.get(&(*a, *b)) {
            println!("Read,{f_path},Start")
        }
        if let Some(f_path) = table.get(b) {
            println!("Read,{f_path},Start");
            println!("Read,MapEnd.tas")
        }
    });
}
