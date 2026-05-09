use rmcp::{ServerHandler, ServiceExt, model::*, schemars, tool, transport::stdio};
use cli_test::analysis::{antipatterns, unused, report::AnalysisReport};
use cli_test::config::Config;
use std::path::Path;

#[derive(Debug, Clone)]
struct CodeAnalyzer;

#[tool(tool_box)]
impl CodeAnalyzer {
    #[tool(description = "Find unused functions in a Python/JS/TS workspace. Returns a JSON array of {file, function, line}.")]
    async fn find_unused_functions(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the workspace root directory")]
        workspace_path: String,
    ) -> String {
        let result = tokio::task::spawn_blocking(move || {
            unused::collect(Path::new(&workspace_path))
        })
        .await
        .unwrap();
        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    #[tool(description = "Find code antipatterns in a workspace: long functions, long parameter lists, duplicate function names, and god classes. Returns JSON with categorized violations.")]
    async fn find_antipatterns(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the workspace root directory")]
        workspace_path: String,
    ) -> String {
        let result = tokio::task::spawn_blocking(move || {
            antipatterns::collect(Path::new(&workspace_path), &Config::load_default())
        })
        .await
        .unwrap();
        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    #[tool(description = "Full static analysis of a workspace: unused functions and all antipatterns combined. Returns JSON with both results.")]
    async fn analyze_workspace(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the workspace root directory")]
        workspace_path: String,
    ) -> String {
        let result = tokio::task::spawn_blocking(move || {
            let p = Path::new(&workspace_path);
            let config = Config::load_default();
            AnalysisReport {
                unused_functions: unused::collect(p),
                antipatterns: antipatterns::collect(p, &config),
            }
        })
        .await
        .unwrap();
        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }
}

#[tool(tool_box)]
impl ServerHandler for CodeAnalyzer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Static analysis server for Python/JS/TS codebases. \
                 Detects unused functions and code antipatterns."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // stdout es el canal JSON-RPC — nunca usar println! aquí
    let service = CodeAnalyzer.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
