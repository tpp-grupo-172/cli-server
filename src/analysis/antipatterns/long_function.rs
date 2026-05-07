use serde_json::Value;
use crate::config::LongFunctionConfig;


pub fn check(data: &Value, file_name: &str, config: &LongFunctionConfig) -> Vec<String> {
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

fn check_one(func: &Value, file_name: &str, config: &LongFunctionConfig) -> Option<String> {
    let name = func.get("name").and_then(|v| v.as_str())?;
    let start = func.get("line").and_then(|v| v.as_u64())?;
    let end = func.get("end_line").and_then(|v| v.as_u64())?;
    let length = end - start;
    
    println!("{} {}", name, length);

    if length > config.max_lines as u64 {
        Some(format!(
            "[LONG FUNCTION]    {:<25} defined in: {:<30} ({} lines)",
            name, file_name, length
        ))
    } else {
        None
    }
}