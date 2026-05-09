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

### Instalación y uso

**Requisitos:** [Rust](https://rust-lang.org/tools/install/) 1.86+

```bash
cargo build --release
cargo run -- <comando> <directorio-del-proyecto>
```

### Comandos

#### `unused` — Funciones sin usar

Detecta funciones que están definidas en el proyecto pero nunca son llamadas desde ningún archivo.

```bash
cargo run -- unused my-project/
```

**Ejemplo de salida:**
```
[UNUSED]           volume                    defined in /home/user/project/main.py
[UNUSED]           hypotenuse                defined in /home/user/project/main.py
```

#### `antipatterns` — Detección de antipatrones

```bash
cargo run -- antipatterns my-project/
```

**Ejemplo de salida:**
```
[LONG FUNCTION]    compute_everything        /home/user/project/main.py (45 lines)
[LONG PARAMS]      process                   /home/user/project/utils.py (7 parameters)
[DUPLICATE NAME]   serialize_packet          defined in: node.py, protocol.py
[GOD CLASS]        Coordinator               defined in: /home/user/project/server.py (score: 0.70, methods: 9, imports: 2, lines: 87)
```

Antipatrones detectados:

- **`[LONG FUNCTION]`** — funciones cuya longitud supera el umbral configurado (default: 30 líneas)
- **`[LONG PARAMS]`** — funciones con más parámetros de los permitidos (default: 5, excluyendo `self`)
- **`[DUPLICATE NAME]`** — mismo nombre definido en múltiples archivos (ignora nombres comunes como `__init__`, `constructor`, etc.)
- **`[GOD CLASS]`** — score ponderado por métodos, imports, líneas y nombre de clase

#### `all` — Todos los análisis

```bash
cargo run -- all my-project/
cargo run -- all my-project/ --json       # salida JSON estructurada
cargo run -- --config MyConfig.toml all my-project/
```

Retorna código `1` si encuentra problemas, `0` si el proyecto está limpio. Útil para CI.

### Configuración

El CLI busca `Config.toml` en el directorio actual, o acepta `--config <path>`:

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

El MCP server siempre usa los valores default hardcodeados (no lee Config.toml).

---

## Build from source

```bash
git clone https://github.com/tpp-grupo-172/cli-test
cd cli-test
cargo build --release
# CLI:        ./target/release/cli-test
# MCP server: ./target/release/mcp-server
```
