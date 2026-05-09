use std::collections::{HashMap, HashSet};
use crate::analysis::FileAnalysis;
use crate::analysis::report::DuplicateViolation;
use crate::config::DuplicateFunctionsConfig;

pub fn check(analyses: &[FileAnalysis], config: &DuplicateFunctionsConfig) -> Vec<DuplicateViolation> {
    let mut violations = vec![];
    let mut seen: HashMap<String, HashSet<String>> = HashMap::new();

    for file in analyses {
        let file_name = file.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if let Some(functions) = file.data.get("functions").and_then(|v| v.as_array()) {
            for func in functions {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    if !config.ignored_names.iter().any(|n| n == name) {
                        seen.entry(name.to_string()).or_default().insert(file_name.clone());
                    }
                }
            }
        }

        if let Some(classes) = file.data.get("classes").and_then(|v| v.as_array()) {
            for class in classes {
                if let Some(methods) = class.get("methods").and_then(|v| v.as_array()) {
                    for method in methods {
                        if let Some(name) = method.get("name").and_then(|v| v.as_str()) {
                            if !config.ignored_names.iter().any(|n| n == name) {
                                seen.entry(name.to_string()).or_default().insert(file_name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    for (name, files) in &seen {
        if files.len() > 1 {
            let mut file_list: Vec<String> = files.iter().cloned().collect();
            file_list.sort();
            violations.push(DuplicateViolation {
                function: name.clone(),
                files: file_list,
            });
        }
    }

    violations
}
