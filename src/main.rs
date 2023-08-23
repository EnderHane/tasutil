use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::{env, fs};

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
        FormatCommand::ScanBreakpoint => _scan_breakpoint(),
    }
}

fn _scan_breakpoint() {
    let (file_num, bp_num) = format::scan_breakpoint(env::current_dir().unwrap())
        .iter()
        .fold((0, 0), |(file_num, bp_num), (s, r)| {
            (
                file_num + 1,
                match r {
                    Ok(map) => {
                        if !map.is_empty() {
                            let p = Path::new(s)
                                .strip_prefix(env::current_dir().unwrap())
                                .unwrap();
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
                },
            )
        });
    println!("Found {bp_num} breakpoints in total {file_num} TAS files")
}

fn _lobby(command: LobbyCommand) {
    match command {
        LobbyCommand::Info { dir } => _info(dir),
        LobbyCommand::Route { dir, num, show_arc } => _route(dir, num, show_arc),
        LobbyCommand::GenerateInput {
            string,
            csv,
            template,
        } => _generate_input(string, csv, template),
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
        let indices = succ
            .iter()
            .flat_map(|((a, b), _)| [*a, *b])
            .collect::<HashSet<_>>();
        let (first, last) = (
            *indices.iter().min().unwrap(),
            *indices.iter().max().unwrap(),
        );
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
            let indices = succ
                .iter()
                .flat_map(|((a, b), _)| [*a, *b])
                .collect::<BTreeSet<_>>();
            let (first, last) = (
                *indices.iter().min().unwrap(),
                *indices.iter().max().unwrap(),
            );
            let buffer_size = num.unwrap_or(1);
            let mut lobby = succ
                .iter()
                .map(|(&(a, b), &(w, _))| ((a, b), w))
                .collect::<BTreeMap<_, _>>();
            // 自动补全重新开始章节的路径
            if let Some(restart_terminal) = lobby.remove(&(first, last)) {
                let keys = lobby.keys().map(|&p| p).collect::<BTreeSet<_>>();
                lobby.extend(
                    indices
                        .iter()
                        .filter(|&&i| i != first && i != last && !keys.contains(&(i, last)))
                        .map(|&i| ((i, last), restart_terminal + 69)),
                )
            }
            let (path_count, results) = lobby::route(&lobby, first, last, buffer_size);
            println!("Found {path_count} paths in {path:?}");
            println!("Best {buffer_size} paths are");
            for (i, (len, p)) in results.iter().enumerate() {
                let path_string = p
                    .iter()
                    .fold::<(String, Option<&u32>), _>(
                        (String::new(), None),
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
                        },
                    )
                    .0;
                println!("{})\t[{}]\t{}", i + 1, len, path_string);
            }
        }
    } else {
        println!("Warning, NO timestamp found in these files, and the algorithm will NOT be run:");
        fail.iter().for_each(|(_, p)| println!("{p}"))
    }
}

fn _generate_input(string: String, csv: PathBuf, template: PathBuf) {
    let csv_content =
        fs::read_to_string(&csv).expect(&format!("Cannot read lobby CSV {}", csv.display()));
    let template_content =
        fs::read_to_string(&template).expect(&format!("Cannot read template {}", csv.display()));
    let table = csv_content
        .lines()
        .filter_map(|l| {
            let pair = l.split(',').collect::<Vec<_>>();
            if pair.len() >= 2 {
                Some((pair[0].trim(), pair[1].trim()))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();
    let route = string.split('-').collect::<Vec<_>>();
    route
        .iter()
        .take(route.len() - 1)
        .zip(route.iter().skip(1))
        .for_each(|(&src, &dst)| {
            let s = template_content.replace("%src%", src).replace("%dst%", dst);
            if let Some(&map) = table.get(dst) {
                println!("{}", s.replace("%map%", map));
            } else {
                println!("{}", s);
            }
        });
}
