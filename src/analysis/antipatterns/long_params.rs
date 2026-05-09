use serde_json::Value;
use crate::config::LongParamsConfig;
use crate::analysis::report::LongParamsViolation;

pub fn check(data: &Value, file_name: &str, config: &LongParamsConfig) -> Vec<LongParamsViolation> {
    let mut violations = vec![];

    if let Some(functions) = data.get("functions").and_then(|v| v.as_array()) {
        for func in functions {
            if let Some(v) = check_one(func, file_name, config) {
                violations.push(v);
            }
        }
    }

    if let Some(classes) = data.get("classes").and_then(|v| v.as_array()) {
        for class in classes {
            if let Some(methods) = class.get("methods").and_then(|v| v.as_array()) {
                for method in methods {
                    if let Some(v) = check_one(method, file_name, config) {
                        violations.push(v);
                    }
                }
            }
        }
    }

    violations
}

fn check_one(func: &Value, file_name: &str, config: &LongParamsConfig) -> Option<LongParamsViolation> {
    let name = func.get("name").and_then(|v| v.as_str())?;
    let params = func.get("parameters").and_then(|v| v.as_array())?;

    let count = params.iter()
        .filter(|p| {
            p.get("name")
                .and_then(|v| v.as_str())
                .map(|n| n != "self")
                .unwrap_or(true)
        })
        .count();

    if count > config.max_params {
        Some(LongParamsViolation {
            file: file_name.to_string(),
            function: name.to_string(),
            param_count: count,
        })
    } else {
        None
    }
}
