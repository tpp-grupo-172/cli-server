mod long_function;
mod long_params;
mod duplicate_functions;
mod god_class;

use std::path::Path;
use crate::config::Config;
use crate::analysis::analyze_project;
use crate::analysis::report::AntipatternReport;

pub fn collect(path: &Path, config: &Config) -> AntipatternReport {
    let analyses = analyze_project(path);
    let mut long_functions = vec![];
    let mut long_params = vec![];

    for file in &analyses {
        let file_path = file.path.display().to_string();

        long_functions.extend(long_function::check(&file.data, &file_path, &config.long_function));
        long_params.extend(long_params::check(&file.data, &file_path, &config.long_params));
    }

    let duplicates = duplicate_functions::check(&analyses, &config.duplicate_functions);
    let god_classes = god_class::check(&analyses, &config.god_class);

    AntipatternReport { long_functions, long_params, duplicates, god_classes }
}

pub fn run(path: &Path, config: &Config, json: bool) {
    let report = collect(path, config);

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return;
    }

    let mut found = false;

    for v in &report.long_functions {
        println!("[LONG FUNCTION]    {:<25} {:<30} ({} lines)", v.function, v.file, v.lines);
        found = true;
    }
    for v in &report.long_params {
        println!("[LONG PARAMS]      {:<25} {:<30} ({} parameters)", v.function, v.file, v.param_count);
        found = true;
    }
    for v in &report.duplicates {
        println!("[DUPLICATE NAME]   {:<25} defined in: {}", v.function, v.files.join(", "));
        found = true;
    }
    for v in &report.god_classes {
        println!(
            "[GOD CLASS]        {:<25} defined in: {:<30} (score: {:.2}, methods: {}, imports: {}, lines: {})",
            v.class, v.file, v.score, v.method_count, v.distinct_imports, v.total_lines
        );
        found = true;
    }

    if !found {
        println!("No antipatterns detected.");
        std::process::exit(0);
    }
    std::process::exit(1);
}
