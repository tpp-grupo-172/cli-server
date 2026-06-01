use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::analysis::{FileAnalysis, analyze_project};
use crate::analysis::report::UnusedFunction;

const JEDI_SCRIPT: &str = include_str!("../jedi_analyzer.py");
const TS_ANALYZER_SCRIPT: &str = include_str!("../ts_analyzer.js");

fn jedi_script_path() -> PathBuf {
    let path = std::env::temp_dir().join("cli_jedi_analyzer.py");
    let _ = std::fs::write(&path, JEDI_SCRIPT);
    path
}

fn ts_analyzer_script_path() -> PathBuf {
    let path = std::env::temp_dir().join("cli_ts_analyzer.js");
    let _ = std::fs::write(&path, TS_ANALYZER_SCRIPT);
    path
}

fn run_jedi_analysis(file_path: &Path) -> HashMap<String, String> {
    if file_path.extension().and_then(|e| e.to_str()) != Some("py") {
        return HashMap::new();
    }
    let script = jedi_script_path();
    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
    match std::process::Command::new("python3").arg(&script).arg(file_path).output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let result: HashMap<String, String> = serde_json::from_str::<HashMap<String, Value>>(&stdout)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(key, val)| {
                    let module_path = val.get("module_path")?.as_str()?.to_string();
                    Some((key, module_path))
                })
                .collect();
            eprintln!("[jedi]  {} → {} type hints", file_name, result.len());
            result
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!("[jedi]  {} → error (exit {}): {}", file_name, out.status, stderr.trim());
            HashMap::new()
        }
        Err(e) => {
            eprintln!("[jedi]  {} → python3 not found: {}", file_name, e);
            HashMap::new()
        }
    }
}

fn run_ts_analysis(file_path: &Path) -> HashMap<String, String> {
    if !matches!(file_path.extension().and_then(|e| e.to_str()), Some("ts" | "tsx" | "js" | "jsx")) {
        return HashMap::new();
    }
    let script = ts_analyzer_script_path();
    match std::process::Command::new("node").arg(&script).arg(file_path).output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let result: HashMap<String, String> = serde_json::from_str::<HashMap<String, Value>>(&stdout)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(key, val)| {
                    let module_path = val.get("module_path")?.as_str()?.to_string();
                    Some((key, module_path))
                })
                .collect();
            result
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            HashMap::new()
        }
        Err(e) => {
            HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionsInFiles {
    pub file_src: String,
    pub function: String,
    pub class_name: Option<String>,
    pub line: i64,
    pub name_start_col: usize,
    pub name_end_col: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Connections {
    pub file_src: String,
    pub file_use: String,
    pub line: i64,
    pub start_col: usize,
    pub end_col: usize,
    pub function: String,
    pub class_name: Option<String>,
}

fn extract_element_type(t: &str) -> Option<String> {
    let t = t.trim();
    if let Some(inner) = t.strip_suffix("[]") {
        return Some(inner.trim().to_string());
    }
    if let Some(inner) = t.strip_prefix("Array<").and_then(|s| s.strip_suffix('>'))
        .or_else(|| t.strip_prefix("ReadonlyArray<").and_then(|s| s.strip_suffix('>'))) {
        return Some(inner.trim().to_string());
    }
    if let Some(inner) = t.strip_prefix("List[").and_then(|s| s.strip_suffix(']')) {
        return Some(inner.trim().to_string());
    }
    if let Some(inner) = t.strip_prefix("Set[").and_then(|s| s.strip_suffix(']')) {
        return Some(inner.trim().to_string());
    }
    None
}

fn extract_type_for_property(return_type: &str, property: &str) -> Option<String> {
    let needle = format!("{}:", property);
    let start = return_type.find(&needle)? + needle.len();
    let rest = return_type[start..].trim_start();
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.').unwrap_or(rest.len());
    let type_name = rest[..end].trim();
    if type_name.is_empty() { None } else { Some(type_name.to_string()) }
}

fn save_function_reference(values: Vec<FileAnalysis>, jedi_types: HashMap<String, String>) -> Vec<Connections> {
    let values_cloned: HashMap<PathBuf, Value> = values
        .iter()
        .map(|f| (f.path.clone(), f.data.clone()))
        .collect();
    let mut connections: Vec<Connections> = vec![];

    for data in &values {
        let path_string = data.path.to_str().unwrap().to_string();
        let binding = data.data.clone();

        let mut imports_hashmap: HashMap<String, String> = HashMap::new();
        let imports = binding
            .get("imports")
            .and_then(|v| v.as_array())
            .expect("imports no es un array");

        for import in imports {
            let name = import.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let path = import.get("path").and_then(|v| v.as_str()).unwrap_or("");
            imports_hashmap.insert(name.to_string(), path.to_string());
        }

        // Helper closure: dado un import_module y un function name,
        // resuelve el return_type buscando en values_cloned
        let resolve_return_type = |import_module: &str, func_name: &str| -> Option<String> {
            let file_path = imports_hashmap.get(import_module)?;
            let file_value = values_cloned.get(&PathBuf::from(file_path))?;

            let functions = file_value.get("functions")?.as_array()?;
            for func in functions {
                if func.get("name")?.as_str()? == func_name {
                    return func.get("return_type")
                        .filter(|v| !v.is_null())
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
            None
        };

        // Helper closure: dado un type_name, encuentra el path del archivo que define esa clase
        let find_class_file = |type_name: &str| -> Option<String> {
            // Normalizar el tipo antes de buscar
            let t = type_name.trim();
            // Stripear notación de array: OrderItem[] → OrderItem
            let t = t.strip_suffix("[]").unwrap_or(t).trim();
            // Tipos primitivos/built-in: nunca van a ser clases de usuario
            const BUILTINS: &[&str] = &[
                "string", "number", "boolean", "void", "any", "unknown", "never",
                "null", "undefined", "object", "symbol", "bigint",
                "str", "int", "float", "bool", "bytes", "NoneType",
                "Function", "Array", "Promise", "Error", "Object",
                "Date", "RegExp", "Map", "Set", "WeakMap", "WeakSet",
            ];
            if BUILTINS.contains(&t) || t.is_empty() { return None; }
            // Parámetros de tipo genérico de una sola letra: T, K, V, etc.
            if t.len() == 1 { return None; }

            for (path, file_value) in &values_cloned {
                let Some(classes) = file_value.get("classes").and_then(|v| v.as_array()) else { continue; };
                for class in classes {
                    let Some(name) = class.get("name").and_then(|v| v.as_str()) else { continue; };
                    if name == t {
                        return path.to_str().map(|s| s.to_string());
                    }
                }
            }
            None
        };

        // Helper closure: dado el archivo que define una clase, su nombre y el nombre de un método,
        // devuelve el return_type de ese método (para resolver cadenas de N niveles).
        let find_method_return_type = |class_file: &str, class_name: &str, method_name: &str| -> Option<String> {
            let file_value = values_cloned.get(&PathBuf::from(class_file))?;
            let classes = file_value.get("classes")?.as_array()?;
            for class in classes {
                if class.get("name")?.as_str()? == class_name {
                    let methods = class.get("methods")?.as_array()?;
                    for method in methods {
                        if method.get("name")?.as_str()? == method_name {
                            return method.get("return_type")
                                .filter(|v| !v.is_null())
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }
            None
        };

        let process_function_calls = |
            function_calls: &Vec<Value>,
            local_variables: &Vec<Value>,
            parameters: &Vec<Value>,
            self_attr_types: &HashMap<String, String>,
            jedi_types: &HashMap<String, String>,
            path_string: &str,
            imports_hashmap: &HashMap<String, String>,
            caller_class: Option<&str>,
        | -> Vec<Connections> {
            let mut new_connections = vec![];

            // ── Pre-pass: construir dos mapas para habilitar resolución de cadenas N-profundas
            let mut call_sources: HashMap<String, String>           = HashMap::new();
            let mut call_contexts: HashMap<String, (String, String)> = HashMap::new();

            for fc in function_calls.iter() {
                let fc_name   = fc.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let fc_import = fc.get("import_name").and_then(|v| v.as_str());
                let fc_chain  = fc.get("chain_source_fn").and_then(|v| v.as_str());
                let fc_object = fc.get("object_name").and_then(|v| v.as_str());

                if fc_chain.is_some() || fc_object.is_some() { continue; }

                if let Some(module) = fc_import {
                    if let Some(src_file) = imports_hashmap.get(module) {
                        call_sources.insert(fc_name.to_string(), src_file.clone());
                        if let Some(rt) = resolve_return_type(module, fc_name) {
                            if let Some(rt_file) = find_class_file(&rt) {
                                call_contexts.insert(fc_name.to_string(), (rt, rt_file));
                            }
                        }
                    }
                } else {
                    let local_fn = binding.get("functions").and_then(|v| v.as_array())
                        .and_then(|fns| fns.iter().find(|f| {
                            f.get("name").and_then(|n| n.as_str()) == Some(fc_name)
                        }));
                    if local_fn.is_some() {
                        call_sources.insert(fc_name.to_string(), path_string.to_string());
                        if let Some(rt) = local_fn
                            .and_then(|f| f.get("return_type"))
                            .filter(|v| !v.is_null())
                            .and_then(|v| v.as_str())
                        {
                            if let Some(rt_file) = find_class_file(rt) {
                                call_contexts.insert(fc_name.to_string(), (rt.to_string(), rt_file));
                            }
                        }
                    }
                }
            }

            // ── Pasada iterativa: resolver cadenas encadenadas de cualquier profundidad
            let mut changed = true;
            while changed {
                changed = false;
                for fc in function_calls.iter() {
                    let fc_name  = fc.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let fc_chain = fc.get("chain_source_fn").and_then(|v| v.as_str());

                    let Some(source_fn) = fc_chain else { continue };
                    if call_sources.contains_key(fc_name) { continue; }

                    if let Some((source_type, source_file)) = call_contexts.get(source_fn).cloned() {
                        call_sources.insert(fc_name.to_string(), source_file.clone());

                        if let Some(rt) = find_method_return_type(&source_file, &source_type, fc_name) {
                            if let Some(rt_file) = find_class_file(&rt) {
                                call_contexts.insert(fc_name.to_string(), (rt, rt_file));
                            }
                        }
                        changed = true;
                    }
                }
            }

            // ── Loop principal: construir Connections usando los mapas pre-computados
            for function_call in function_calls {
                let name        = function_call.get("name").and_then(|v| v.as_str()).unwrap_or("<sin nombre>");
                let import_name = function_call.get("import_name").and_then(|v| v.as_str());
                let object_name = function_call.get("object_name").and_then(|v| v.as_str());
                let chain_source_fn = function_call.get("chain_source_fn").and_then(|v| v.as_str());
                let line      = function_call.get("line").and_then(|v| v.as_i64()).unwrap_or(0);
                let start_col = function_call.get("start_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let end_col   = function_call.get("end_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                if let Some(import_module) = import_name {
                    // Caso 1: llamada directa a función importada
                    if let Some(path) = imports_hashmap.get(import_module) {
                        new_connections.push(Connections {
                            file_src: path.clone(), file_use: path_string.to_string(),
                            line, start_col, end_col, function: name.to_string(),
                            class_name: None,
                        });
                    }
                } else if let Some(obj_name) = object_name {
                    // Caso 2: método sobre variable  →  obj.method()
                    let jedi_lookup = |lookup_name: &str| -> Option<String> {
                        let key = format!("{}@{}", lookup_name, line);
                        jedi_types.get(&key).cloned()
                    };

                    let connections_before = new_connections.len();

                    if obj_name == "self" || obj_name == "this" {
                        // 2a: self.funcion_de_clase() — mismo archivo
                        new_connections.push(Connections {
                            file_src: path_string.to_string(), file_use: path_string.to_string(),
                            line, start_col, end_col, function: name.to_string(),
                            class_name: caller_class.map(|s| s.to_string()),
                        });
                    } else if let Some(attr_name) = obj_name.strip_prefix("self.")
                        .or_else(|| obj_name.strip_prefix("this."))
                    {
                        // 2b: self.order_service.funcion1() — tipo desde __init__ o jedi
                        if let Some(type_name) = self_attr_types.get(attr_name) {
                            if let Some(class_file) = find_class_file(type_name) {
                                new_connections.push(Connections {
                                    file_src: class_file, file_use: path_string.to_string(),
                                    line, start_col, end_col, function: name.to_string(),
                                    class_name: Some(type_name.clone()),
                                });
                            }
                        }
                        // 2b-fallback jedi
                        if new_connections.len() == connections_before {
                            if let Some(module_path) = jedi_lookup(attr_name) {
                                new_connections.push(Connections {
                                    file_src: module_path, file_use: path_string.to_string(),
                                    line, start_col, end_col, function: name.to_string(),
                                    class_name: None,
                                });
                            }
                        }
                    } else {
                        // 2c: variable local asignada desde una función
                        // 2c-path-0: variable de loop/generator cuyo iterable es un atributo tipado
                        let iterated_from = local_variables.iter()
                            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(obj_name))
                            .and_then(|v| v.get("iterated_from"))
                            .and_then(|v| v.as_str());

                        if let Some(iterable) = iterated_from {
                            let attr_name = iterable.strip_prefix("self.")
                                .or_else(|| iterable.strip_prefix("this."));
                            if let Some(attr) = attr_name {
                                if let Some(collection_type) = self_attr_types.get(attr) {
                                    let element_type = extract_element_type(collection_type)
                                        .or_else(|| {
                                            let t = collection_type.trim();
                                            t.strip_prefix("Tuple[").and_then(|s| s.strip_suffix(']'))
                                                .and_then(|inner| inner.split(',').next())
                                                .map(|s| s.trim().to_string())
                                        });
                                    if let Some(elem_type) = element_type {
                                        let elem_type = elem_type.as_str();
                                        if let Some(class_file) = find_class_file(elem_type) {
                                            new_connections.push(Connections {
                                                file_src: class_file, file_use: path_string.to_string(),
                                                line, start_col, end_col, function: name.to_string(),
                                                class_name: Some(elem_type.to_string()),
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        let local_var = local_variables.iter()
                            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(obj_name));
                        let assigned_from = local_var
                            .and_then(|v| v.get("assigned_from"))
                            .and_then(|v| v.as_str());
                        let destructured_property = local_var
                            .and_then(|v| v.get("destructured_property"))
                            .and_then(|v| v.as_str());

                        if let Some(assigned_func) = assigned_from {
                            let source_import = function_calls.iter()
                                .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(assigned_func))
                                .and_then(|c| c.get("import_name"))
                                .and_then(|v| v.as_str());

                            if let Some(module) = source_import {
                                // 2c-path-1: la función vino de un import directo
                                if let Some(raw_return_type) = resolve_return_type(module, assigned_func) {
                                    let resolved_type = destructured_property
                                        .and_then(|prop| extract_type_for_property(&raw_return_type, prop))
                                        .unwrap_or(raw_return_type);
                                    if let Some(class_file) = find_class_file(&resolved_type) {
                                        new_connections.push(Connections {
                                            file_src: class_file, file_use: path_string.to_string(),
                                            line, start_col, end_col, function: name.to_string(),
                                            class_name: Some(resolved_type.clone()),
                                        });
                                    }
                                }
                            } else if destructured_property.is_some() {
                                // 2c-path-1b: función local (mismo archivo) con destructuring
                                let source_fn_node = binding.get("functions")
                                    .and_then(|v| v.as_array())
                                    .and_then(|funcs| funcs.iter().find(|f| {
                                        f.get("name").and_then(|n| n.as_str()) == Some(assigned_func)
                                    }));
                                let local_return_type = source_fn_node
                                    .and_then(|f| f.get("return_type"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                if let Some(raw_return_type) = local_return_type {
                                    if let Some(prop_type) = extract_type_for_property(&raw_return_type, destructured_property.unwrap()) {
                                        if let Some(class_file) = find_class_file(&prop_type) {
                                            new_connections.push(Connections {
                                                file_src: class_file, file_use: path_string.to_string(),
                                                line, start_col, end_col, function: name.to_string(),
                                                class_name: Some(prop_type.clone()),
                                            });
                                        }
                                    }
                                } else if new_connections.len() == connections_before {
                                    // 2c-path-1b-fallback
                                    let prop = destructured_property.unwrap();
                                    let inferred_type = source_fn_node
                                        .and_then(|f| f.get("local_variables"))
                                        .and_then(|v| v.as_array())
                                        .and_then(|vars| vars.iter().find(|v| {
                                            v.get("name").and_then(|n| n.as_str()) == Some(prop)
                                        }))
                                        .and_then(|v| v.get("assigned_from"))
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    if let Some(type_name) = inferred_type {
                                        if let Some(class_file) = find_class_file(&type_name) {
                                            new_connections.push(Connections {
                                                file_src: class_file, file_use: path_string.to_string(),
                                                line, start_col, end_col, function: name.to_string(),
                                                class_name: Some(type_name.clone()),
                                            });
                                        }
                                    }
                                }
                            } else {
                                // 2c-path-2: la función fue llamada sobre self.attr o this.attr
                                const ARRAY_ELEMENT_METHODS: &[&str] = &[
                                    "shift", "pop", "find", "findLast", "at", "get", "first", "last",
                                ];

                                let via_self_attr: Option<(String, String)> = function_calls.iter()
                                    .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(assigned_func))
                                    .and_then(|c| {
                                        let obj = c.get("object_name").and_then(|v| v.as_str())?;
                                        let attr = obj.strip_prefix("self.").or_else(|| obj.strip_prefix("this."))?;
                                        let attr_type = self_attr_types.get(attr)?;

                                        if ARRAY_ELEMENT_METHODS.contains(&assigned_func) {
                                            if let Some(elem_type) = extract_element_type(attr_type) {
                                                let f = find_class_file(&elem_type)?;
                                                return Some((elem_type, f));
                                            }
                                        }

                                        let class_file = find_class_file(attr_type)?;
                                        let return_type = find_method_return_type(&class_file, attr_type, assigned_func)?;
                                        let base = {
                                            let t = return_type.trim();
                                            if let Some(inner) = t.strip_prefix("Optional[").and_then(|s| s.strip_suffix(']')) {
                                                inner.trim().to_string()
                                            } else if let Some(b) = t.split('|').next() {
                                                b.trim().to_string()
                                            } else {
                                                t.to_string()
                                            }
                                        };
                                        find_class_file(&base).map(|f| (base, f))
                                    });
                                if let Some((callee_class, class_file)) = via_self_attr {
                                    new_connections.push(Connections {
                                        file_src: class_file, file_use: path_string.to_string(),
                                        line, start_col, end_col, function: name.to_string(),
                                        class_name: Some(callee_class),
                                    });
                                }
                            }
                        } else {
                            // 2d: parámetro con tipo anotado
                            let param_type = parameters.iter()
                                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(obj_name))
                                .and_then(|p| p.get("param_type"))
                                .and_then(|v| v.as_str());

                            if let Some(type_annotation) = param_type {
                                let base_type = {
                                    let t = type_annotation.trim().trim_matches('"').trim_matches('\'');
                                    if let Some(inner) = t.strip_prefix("Optional[").and_then(|s| s.strip_suffix(']')) {
                                        inner.trim()
                                    } else if let Some(base) = t.split('|').next() {
                                        base.trim()
                                    } else {
                                        t
                                    }
                                };
                                if let Some(class_file) = find_class_file(base_type) {
                                    new_connections.push(Connections {
                                        file_src: class_file, file_use: path_string.to_string(),
                                        line, start_col, end_col, function: name.to_string(),
                                        class_name: Some(base_type.to_string()),
                                    });
                                }
                            }
                        }

                        // 2e: fallback jedi para variable local/parámetro no resuelto
                        if new_connections.len() == connections_before {
                            if let Some(module_path) = jedi_lookup(obj_name) {
                                new_connections.push(Connections {
                                    file_src: module_path, file_use: path_string.to_string(),
                                    line, start_col, end_col, function: name.to_string(),
                                    class_name: None,
                                });
                            }
                        }
                    }
                } else if let Some(source_fn) = chain_source_fn {
                    // Caso 3: llamada encadenada
                    if let Some(src_file) = call_sources.get(source_fn) {
                        new_connections.push(Connections {
                            file_src: src_file.clone(), file_use: path_string.to_string(),
                            line, start_col, end_col, function: name.to_string(),
                            class_name: call_contexts.get(source_fn).map(|(t, _)| t.clone()),
                        });
                    }
                } else {
                    // Caso 4: llamada local directa (misma función en mismo archivo)
                    let defined_in_same_file = binding.get("functions").and_then(|v| v.as_array())
                        .map(|funcs| funcs.iter().any(|f| {
                            f.get("name").and_then(|n| n.as_str()) == Some(name)
                        }))
                        .unwrap_or(false);

                    if defined_in_same_file {
                        new_connections.push(Connections {
                            file_src: path_string.to_string(), file_use: path_string.to_string(),
                            line, start_col, end_col, function: name.to_string(),
                            class_name: None,
                        });
                    }
                }
            }

            new_connections
        };

        // Procesar clases
        let classes = binding
            .get("classes")
            .and_then(|v| v.as_array())
            .expect("classes no es un array");

        for class in classes {
            let current_class_name = class.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let methods = class
                .get("methods")
                .and_then(|v| v.as_array())
                .expect("methods no es un array");

            // Construir mapa atributo_de_instancia → tipo
            let self_attr_types: HashMap<String, String> = {
                let empty: Vec<Value> = vec![];
                let mut map: HashMap<String, String> = HashMap::new();

                // Fuente 2: field annotations declaradas en el cuerpo de la clase
                for field in class.get("fields").and_then(|v| v.as_array()).unwrap_or(&empty) {
                    if let (Some(fname), Some(ann)) = (
                        field.get("name").and_then(|v| v.as_str()),
                        field.get("annotation").and_then(|v| v.as_str()),
                    ) {
                        map.insert(fname.to_string(), ann.to_string());
                    }
                }

                // Fuente 1: __init__ / constructor (sobreescribe fields si hay conflicto)
                if let Some(init_method) = methods.iter().find(|m| {
                    matches!(m.get("name").and_then(|v| v.as_str()), Some("__init__") | Some("constructor"))
                }) {
                    let param_types: HashMap<String, String> = init_method
                        .get("parameters").and_then(|v| v.as_array()).unwrap_or(&empty)
                        .iter()
                        .filter_map(|p| Some((
                            p.get("name").and_then(|v| v.as_str())?.to_string(),
                            p.get("param_type").and_then(|v| v.as_str())?.to_string(),
                        )))
                        .collect();

                    for lv in init_method.get("local_variables").and_then(|v| v.as_array()).unwrap_or(&empty) {
                        let lv_name = lv.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let attr = lv_name.strip_prefix("self.").or_else(|| lv_name.strip_prefix("this."));
                        if let Some(attr) = attr {
                            if let Some(assigned_id) = lv.get("assigned_identifier").and_then(|v| v.as_str()) {
                                if let Some(param_type) = param_types.get(assigned_id) {
                                    map.insert(attr.to_string(), param_type.clone());
                                }
                            }
                        }
                    }
                }

                map
            };

            for method in methods {
                let function_calls = method
                    .get("function_calls")
                    .and_then(|v| v.as_array())
                    .expect("function_calls no es un array");
                let local_variables = method
                    .get("local_variables")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let method_parameters = method
                    .get("parameters")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                let new_connections = process_function_calls(
                    function_calls,
                    &local_variables,
                    &method_parameters,
                    &self_attr_types,
                    &jedi_types,
                    &path_string,
                    &imports_hashmap,
                    Some(current_class_name.as_str()),
                );

                connections.extend(new_connections);
            }
        }

        // Procesar funciones top-level
        let functions = binding
            .get("functions")
            .and_then(|v| v.as_array())
            .expect("functions no es un array");

        for func in functions {
            let function_calls = func
                .get("function_calls")
                .and_then(|v| v.as_array())
                .expect("function_calls no es un array");
            let local_variables = func
                .get("local_variables")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let func_parameters = func
                .get("parameters")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let new_connections = process_function_calls(
                function_calls,
                &local_variables,
                &func_parameters,
                &HashMap::new(),
                &jedi_types,
                &path_string,
                &imports_hashmap,
                None,
            );

            connections.extend(new_connections);
        }
    }

    connections
}

fn save_functions(values: Vec<FileAnalysis>) -> Vec<FunctionsInFiles> {
    let mut functions_in_file: Vec<FunctionsInFiles> = vec![];

    for data in values {
        let path_string = data.path.to_str().unwrap().to_string();
        let binding = data.data.clone();

        let calsses = binding
            .get("classes")
            .and_then(|v| v.as_array())
            .expect("classes no es un array");

        for calss in calsses {
            let class_name_str = calss.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
            let methods = calss
                .get("methods")
                .and_then(|v| v.as_array())
                .expect("methods no es un array");

            for method in methods {
                if let Some(function_name) = method.get("name").and_then(|v| v.as_str()) {
                    let line = method.get("line").and_then(|v| v.as_i64()).unwrap_or(1);
                    let name_start_col = method.get("name_start_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let name_end_col = method.get("name_end_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    functions_in_file.push(FunctionsInFiles {
                        file_src: path_string.clone(),
                        function: function_name.to_string(),
                        class_name: class_name_str.clone(),
                        line,
                        name_start_col,
                        name_end_col,
                    });
                }
            }
        }

        let functions = binding
            .get("functions")
            .and_then(|v| v.as_array())
            .expect("functions no es un array");

        for function in functions {
            if let Some(function_name) = function.get("name").and_then(|v| v.as_str()) {
                let line = function.get("line").and_then(|v| v.as_i64()).unwrap_or(1);
                let name_start_col = function.get("name_start_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let name_end_col = function.get("name_end_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                functions_in_file.push(FunctionsInFiles {
                    file_src: path_string.clone(),
                    function: function_name.to_string(),
                    class_name: None,
                    line,
                    name_start_col,
                    name_end_col,
                });
            }
        }
    }

    functions_in_file
}

pub fn find_unused_functions(
    functions_in_files: &[FunctionsInFiles],
    connections: &[Connections],
) -> Vec<FunctionsInFiles> {
    let used: HashSet<(&str, &str)> = connections
        .iter()
        .map(|c| (c.file_src.as_str(), c.function.as_str()))
        .collect();

    functions_in_files
        .iter()
        .filter(|f| f.function != "main" && f.function != "constructor" && !f.function.starts_with('_'))
        .filter(|f| !used.contains(&(f.file_src.as_str(), f.function.as_str())))
        .cloned()
        .collect()
}

pub fn collect(path: &Path) -> Vec<UnusedFunction> {
    let analyses = analyze_project(path);
    let mut jedi_types: HashMap<String, String> = HashMap::new();
    for analysis in &analyses {
        jedi_types.extend(run_jedi_analysis(&analysis.path));
        jedi_types.extend(run_ts_analysis(&analysis.path));
    }
    let connections = save_function_reference(analyses.clone(), jedi_types);
    let functions_in_files = save_functions(analyses);
    find_unused_functions(&functions_in_files, &connections)
        .into_iter()
        .map(|f| UnusedFunction { file: f.file_src, function: f.function, line: f.line })
        .collect()
}

pub fn run(path: &Path, json: bool) -> bool {
    let result = collect(path);
    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        if result.is_empty() {
            println!("No unused functions detected.");
            return false
        }
        for f in &result {
            println!("[UNUSED]           {:<25} defined in {}", f.function, f.file);
        }
    }
    return true
}
