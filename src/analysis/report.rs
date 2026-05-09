use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LongFunctionViolation {
    pub file: String,
    pub function: String,
    pub lines: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LongParamsViolation {
    pub file: String,
    pub function: String,
    pub param_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DuplicateViolation {
    pub function: String,
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GodClassViolation {
    pub file: String,
    pub class: String,
    pub score: f32,
    pub method_count: usize,
    pub distinct_imports: usize,
    pub total_lines: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnusedFunction {
    pub file: String,
    pub function: String,
    pub line: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AntipatternReport {
    pub long_functions: Vec<LongFunctionViolation>,
    pub long_params: Vec<LongParamsViolation>,
    pub duplicates: Vec<DuplicateViolation>,
    pub god_classes: Vec<GodClassViolation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnalysisReport {
    pub unused_functions: Vec<UnusedFunction>,
    pub antipatterns: AntipatternReport,
}
