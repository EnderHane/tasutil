use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::io::Result;
use walkdir::WalkDir;

pub fn scan_breakpoint<P: AsRef<Path>>(path: P) -> HashMap<String, Result<BTreeMap<u32, String>>> {
    WalkDir::new(path).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tas"))
        .map(|e| (e.path().to_string_lossy().to_string(), fs::read_to_string(e.path()).map(|s| {
            s.lines()
                .enumerate()
                .filter_map(|(n, l)| l.contains("***").then_some((n as u32, l.to_string())))
                .collect()
        })))
        .collect()
}