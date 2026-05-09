pub mod report;
pub mod unused;
pub mod antipatterns;

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tree_sitter_test::run_analysis;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileAnalysis {
    pub path: PathBuf,
    pub data: Value,
}

pub fn analyze_project(project_path: &Path) -> Vec<FileAnalysis> {
    let supported_extensions = ["py", "ts", "tsx", "js", "jsx"];
    let mut results = vec![];
    let mut dirs = vec![project_path.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    dirs.push(path);
                }
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !supported_extensions.contains(&ext) {
                continue;
            }

            match run_analysis(&path, &[project_path.to_path_buf()]) {
                Ok(json_str) => {
                    match serde_json::from_str::<Value>(&json_str) {
                        Ok(data) => results.push(FileAnalysis { path, data }),
                        Err(e) => eprintln!("Warning: could not parse analysis for {}: {}", path.display(), e),
                    }
                }
                Err(e) => eprintln!("Warning: could not analyze {}: {}", path.display(), e),
            }
        }
    }

    results
}