use std::cmp::Reverse;
use std::collections::{BinaryHeap, BTreeMap, BTreeSet};
use std::fs;
use std::ops::Not;
use std::path::Path;

use lazy_static::lazy_static;
use regex::Regex;
use walkdir::WalkDir;


pub fn lobby_map<P: AsRef<Path>>(path: P)
    -> (BTreeMap<(u32, u32), (Option<u32>, String)>, BTreeMap<(u32, u32), (Option<u32>, String)>){
    lazy_static! {
        static ref FILE_NAME_PATTERN: Regex = Regex::new(r"^[[:alpha:]]+_([[:digit:]]+)\-([[:digit:]]+)\.tas$").unwrap();
        static ref TIMESTAMP_PATTERN: Regex = Regex::new(r"[[:digit:].:]+\(([[:digit:]]+)\)").unwrap();
    }
    WalkDir::new(path).max_depth(1).into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            FILE_NAME_PATTERN.captures(e.file_name().to_string_lossy().to_string().as_str())
                .map(|cap| (
                    (
                        cap.get(1).unwrap().as_str().parse().unwrap(),
                        cap.get(2).unwrap().as_str().parse().unwrap()
                    ),
                    e.path().to_string_lossy().to_string())
                )
        })
        .map(|(edge, file_path)| (
            edge,
            (
                fs::read_to_string(file_path.as_str()).ok().and_then(|s|
                    s.lines().find_map(|line|
                        TIMESTAMP_PATTERN.captures(line)
                            .map(|cap| cap.get(1).unwrap().as_str().to_string())
                    ).and_then(|dec| dec.parse().ok())),
                file_path
            )
        ))
        .partition::<BTreeMap<_, _>, _>(|(_, (w, _))| w.is_some())
}


pub fn route(
    lobby: &BTreeMap<(u32, u32), u32>,
    src: &u32,
    dst: &u32,
    buffer_size: &u32
) -> (u32, Vec<Reverse<(u32, Vec<u32>)>>) {
    let graph = lobby.iter()
        .flat_map(|((a, b), _)| [*a, *b])
        .collect::<BTreeSet<_>>().iter()
        .map(|i| (*i, lobby.iter()
            .filter(|((a, _), _)| a == i)
            .map(|((_, b), w)| (*b, *w))
            .collect::<BTreeMap<_, _>>()
        ))
        .collect::<BTreeMap<_, _>>();

    let mut path_count = 0;
    let mut result_buffer: BinaryHeap<Reverse<(u32, Vec<u32>)>> = BinaryHeap::new();

    fn search(
        graph: &BTreeMap<u32, BTreeMap<u32, u32>>,
        current_vertex: &u32,
        destination: &u32,
        path_stack: &mut Vec<u32>,
        current_length: &mut u32,
        path_count: &mut u32,
        result_buffer: &mut BinaryHeap<Reverse<(u32, Vec<u32>)>>,
        buffer_size: &u32
    ) {
        if path_stack.len() >= graph.len() - 1 {
            if let Some(adj) = graph.get(current_vertex) {
                if let Some(w) = adj.get(destination) {
                    path_stack.push(*destination);
                    *current_length += w;
                    result_buffer.push(Reverse((*current_length, path_stack.clone())));
                    *path_count += 1;
                    if result_buffer.len() > *buffer_size as usize {
                        result_buffer.pop();
                    }
                    *current_length -= w;
                    path_stack.pop();
                }
            }
        } else if let Some(adj) = graph.get(current_vertex) {
            for (next, w) in adj {
                if next != destination && path_stack.contains(next).not() {
                    path_stack.push(*next);
                    *current_length += w;
                    search(
                        graph,
                        next,
                        destination,
                        path_stack,
                        current_length,
                        path_count,
                        result_buffer,
                        buffer_size
                    );
                    *current_length -= w;
                    path_stack.pop();
                }
            }
        }
    }
    search(
        &graph,
        src,
        dst,
        &mut vec![*src],
        &mut 0,
        &mut path_count,
        &mut result_buffer,
        buffer_size
    );

    (path_count, result_buffer.into_sorted_vec())
}
