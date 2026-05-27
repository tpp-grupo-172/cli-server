use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::analysis::{FileAnalysis, analyze_project};
use crate::analysis::report::UnusedFunction;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionsInFiles {
    pub file_src: String,
    pub function: String,
    pub line: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Connections {
    pub file_src: String,
    pub file_use: String,
    pub line: i64,
    pub function: String,
}

fn save_function_reference(values: Vec<FileAnalysis>) -> Vec<Connections> {
    let values_cloned: HashMap<PathBuf, Value> = values
        .iter()
        .map(|f| (f.path.clone(), f.data.clone()))
        .collect();
    let mut connections: Vec<Connections> = vec![];

    for data in values {
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
        // resuelve el return_type buscando en el store
        let resolve_return_type = |import_module: &str, func_name: &str| -> Option<String> {
            let file_path = imports_hashmap.get(import_module)?;
            let file_value = values_cloned.get(&PathBuf::from(file_path))?;
            
            // buscar en funciones top-level
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
            for (path, file_value) in &values_cloned {
                let classes = file_value.get("classes")?.as_array()?;
                for class in classes {
                    if class.get("name")?.as_str()? == type_name {
                        return Some(path.to_str()?.to_string());
                    }
                }
            }
            None
        };


        let process_function_calls = |
            function_calls: &Vec<Value>,
            local_variables: &Vec<Value>,
            path_string: &str,
            imports_hashmap: &HashMap<String, String>,
        | -> Vec<Connections> {
            let mut new_connections = vec![];
    
            for function_call in function_calls {
                let name = function_call
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<sin nombre>");
                let import_name = function_call
                    .get("import_name")
                    .and_then(|v| v.as_str());
                let object_name = function_call
                    .get("object_name")
                    .and_then(|v| v.as_str());
                let line = function_call
                    .get("line")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
    
                if let Some(import_module) = import_name {
                    // Caso normal — import directo
                    if let Some(path) = imports_hashmap.get(import_module) {
                        new_connections.push(Connections {
                            file_src: path.clone(),
                            file_use: path_string.to_string(),
                            line,
                            function: name.to_string(),
                        });
                    }
                } else if let Some(obj_name) = object_name {
                    // Caso nuevo — resolver via local_variables + return_type + clase
                    
                    // 1. Buscar assigned_from en local_variables
                    let assigned_from = local_variables
                        .iter()
                        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(obj_name))
                        .and_then(|v| v.get("assigned_from"))
                        .and_then(|v| v.as_str());
    
                    if let Some(assigned_func) = assigned_from {
                        // 2. Buscar de qué módulo viene assigned_func
                        let source_import = function_calls
                            .iter()
                            .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(assigned_func))
                            .and_then(|c| c.get("import_name"))
                            .and_then(|v| v.as_str());
    
                        if let Some(import_module) = source_import {
                            // 3. Resolver return_type de assigned_func
                            if let Some(return_type) = resolve_return_type(import_module, assigned_func) {
                                // 4. Buscar el archivo que define esa clase
                                if let Some(class_file) = find_class_file(&return_type) {
                                    new_connections.push(Connections {
                                        file_src: class_file,
                                        file_use: path_string.to_string(),
                                        line,
                                        function: name.to_string(),
                                    });
                                }
                            }
                        }
                    }
                } else {
                    let defined_in_same_file = binding
                        .get("functions")
                        .and_then(|v| v.as_array())
                        .map(|funcs| funcs.iter().any(|f| {
                            f.get("name").and_then(|n| n.as_str()) == Some(name)
                        }))
                        .unwrap_or(false);
    
                    if defined_in_same_file {
                        new_connections.push(Connections {
                            file_src: path_string.to_string(),
                            file_use: path_string.to_string(),
                            line,
                            function: name.to_string(),
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
            let methods = class
                .get("methods")
                .and_then(|v| v.as_array())
                .expect("methods no es un array");
    
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
    
                let new_connections = process_function_calls(
                    function_calls,
                    &local_variables,
                    &path_string,
                    &imports_hashmap,
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
    
            let new_connections = process_function_calls(
                function_calls,
                &local_variables,
                &path_string,
                &imports_hashmap,
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
            let methods = calss
                .get("methods")
                .and_then(|v| v.as_array())
                .expect("methods no es un array");
    
            for method in methods {
                if let Some(function_name) = method.get("name").and_then(|v| v.as_str()) {
    
                    let line = method
                        .get("line")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1);
    
                    let cloned_path = path_string.clone();
                    let f_in_files = FunctionsInFiles {
                        file_src: cloned_path,
                        function: function_name.to_string(),
                        line: line
                    };
    
                    functions_in_file.push(f_in_files);
                }
            }
        }

        let functions = binding
            .get("functions")
            .and_then(|v| v.as_array())
            .expect("functions no es un array");
    
        for function in functions {
            if let Some(function_name) = function.get("name").and_then(|v| v.as_str()) {
    
                let line = function
                  .get("line")
                  .and_then(|v| v.as_i64())
                  .unwrap_or(1);
                let cloned_path = path_string.clone();
                let f_in_files = FunctionsInFiles {
                    file_src: cloned_path,
                    function: function_name.to_string(),
                    line: line
                };
    
                functions_in_file.push(f_in_files);
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
        .filter(|f| f.function != "main" && !f.function.starts_with('_'))
        .filter(|f| !used.contains(&(f.file_src.as_str(), f.function.as_str())))
        .cloned()
        .collect()
}

pub fn collect(path: &Path) -> Vec<UnusedFunction> {
    let analyses = analyze_project(path);
    let connections = save_function_reference(analyses.clone());
    let functions_in_files = save_functions(analyses);
    find_unused_functions(&functions_in_files, &connections)
        .into_iter()
        .map(|f| UnusedFunction { file: f.file_src, function: f.function, line: f.line })
        .collect()
}

pub fn run(path: &Path, json: bool) {
    let result = collect(path);
    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        for f in &result {
            println!("[UNUSED]           {:<25} defined in {}", f.function, f.file);
        }
        std::process::exit(1);
    }
    std::process::exit(0);
}