# cli-test — Static analysis CLI + MCP server

Static analysis tool for Python/JS/TS codebases. Detects unused functions and code antipatterns. Can be used as a CLI or as an MCP server with Claude Desktop.

## MCP server setup

### 1. Install

```bash
cargo install --git https://github.com/tpp-grupo-172/cli-test --bin mcp-server
```

Requires Rust 1.86+. The binary is placed at `~/.cargo/bin/mcp-server`.

### 2. Configure Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (create it if it doesn't exist):

```json
{
  "mcpServers": {
    "code-analyzer": {
      "command": "/Users/YOUR_USERNAME/.cargo/bin/mcp-server"
    }
  }
}
```

Replace `YOUR_USERNAME` with your macOS username (run `whoami` if unsure). Then restart Claude Desktop.

### 3. Use in Claude

The server exposes three tools you can ask Claude to use:

| Tool | What it does | Parameter |
|---|---|---|
| `find_unused_functions` | Functions defined but never called | `workspace_path` — absolute path to the project root |
| `find_antipatterns` | Long functions, god classes, long param lists, duplicate names | `workspace_path` |
| `analyze_workspace` | Both analyses combined | `workspace_path` |

Example prompt: *"Use find_unused_functions on /Users/me/myproject and tell me what to clean up."*

### Verify the server works (optional)

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' \
  | ~/.cargo/bin/mcp-server
```

You should see a JSON response listing the three tools.

---

## CLI usage

```bash
cli-test unused <path>           # list unused functions
cli-test antipatterns <path>     # list antipattern violations
cli-test all <path>              # both
cli-test all <path> --json       # machine-readable JSON output
cli-test all <path> --config MyConfig.toml  # custom thresholds
```

Exit codes: `antipatterns` and `all` exit `1` when violations are found (useful for CI), `0` when clean.

### Configuration

Drop a `Config.toml` in the directory where you run the CLI, or pass `--config <path>`:

```toml
[long_function]
max_lines = 30

[long_params]
max_params = 5

[god_class]
flag_threshold = 0.5
method_count_norm = 8.0
distinct_imports_norm = 4.0
total_lines_norm = 150.0
weight_method_count = 0.50
weight_distinct_imports = 0.15
weight_total_lines = 0.30
weight_name = 0.05
god_names = ["manager", "coordinator", "handler", "controller", "processor"]

[duplicate_functions]
ignored_names = ["__init__", "constructor", "run", "main"]
```

The MCP server always uses the hardcoded defaults above (Config.toml is not loaded).

---

## Build from source

```bash
git clone https://github.com/tpp-grupo-172/cli-test
cd cli-test
cargo build --release
# CLI:        ./target/release/cli-test
# MCP server: ./target/release/mcp-server
```
