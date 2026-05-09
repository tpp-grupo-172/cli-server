use serde_json::Value;
use std::collections::HashSet;
use crate::analysis::FileAnalysis;
use crate::analysis::report::GodClassViolation;
use crate::config::GodClassConfig;

pub fn check(analyses: &[FileAnalysis], config: &GodClassConfig) -> Vec<GodClassViolation> {
    let mut violations = vec![];

    for file in analyses {
        let file_name = file.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if let Some(classes) = file.data.get("classes").and_then(|v| v.as_array()) {
            for class in classes {
                if let Some(result) = check_one(class, &file_name, config) {
                    violations.push(result);
                }
            }
        }
    }

    violations
}

fn check_one(class: &Value, file_name: &str, config: &GodClassConfig) -> Option<GodClassViolation> {
    let class_name = class.get("name").and_then(|v| v.as_str())?.to_string();
    let methods = class.get("methods").and_then(|v| v.as_array())?;

    let method_count = methods.len();
    let distinct_imports = count_distinct_imports(methods);
    let total_lines = sum_method_lines(methods);

    let score = compute_score(method_count, distinct_imports, total_lines, &class_name, config);

    if score >= config.flag_threshold {
        Some(GodClassViolation {
            file: file_name.to_string(),
            class: class_name,
            score,
            method_count,
            distinct_imports,
            total_lines,
        })
    } else {
        None
    }
}

fn compute_score(
    method_count: usize,
    distinct_imports: usize,
    total_lines: usize,
    class_name: &str,
    config: &GodClassConfig,
) -> f32 {
    let method_score = (method_count as f32 / config.method_count_norm).min(1.0);
    let imports_score = (distinct_imports as f32 / config.distinct_imports_norm).min(1.0);
    let lines_score = (total_lines as f32 / config.total_lines_norm).min(1.0);
    let name_score = if has_god_name(class_name, config) { 1.0 } else { 0.0 };

    config.weight_method_count * method_score
        + config.weight_distinct_imports * imports_score
        + config.weight_total_lines * lines_score
        + config.weight_name * name_score
}

fn count_distinct_imports(methods: &[Value]) -> usize {
    let mut seen = HashSet::new();
    for method in methods {
        if let Some(calls) = method.get("function_calls").and_then(|v| v.as_array()) {
            for call in calls {
                if let Some(import_name) = call.get("import_name").and_then(|v| v.as_str()) {
                    if !import_name.is_empty() {
                        seen.insert(import_name.to_string());
                    }
                }
            }
        }
    }
    seen.len()
}

fn sum_method_lines(methods: &[Value]) -> usize {
    methods.iter().map(|m| {
        let start = m.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
        let end = m.get("end_line").and_then(|v| v.as_u64()).unwrap_or(0);
        end.saturating_sub(start) as usize
    }).sum()
}

fn has_god_name(class_name: &str, config: &GodClassConfig) -> bool {
    let lower = class_name.to_lowercase();
    config.god_names.iter().any(|n| lower.contains(n))
}
