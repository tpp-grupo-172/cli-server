# CLI-Server

Herramienta de análisis estático para proyectos de código fuente, en busca de funciones sin usar y antipatrones de diseño. Puede utilizarse como CLI o como servidor MCP compatible con Claude Desktop. Actualmente soporta Python, JavaScript y TypeScript.

## Instalación y uso

### Requisitos

- [Rust](https://rust-lang.org/tools/install/)
- Cargo (incluido con Rust)
- El repositorio tree-sitter-test clonado localmente (referenciado por path en Cargo.toml)

### Compilación

```bash
cargo build --release
```

### Binarios generados

```
CLI: ./target/release/cli-server
MCP server: ./target/release/mcp-server
```

---

## Uso como MCP

### Instalación

```bash
cargo install --git https://github.com/tpp-grupo-172/cli-server --bin mcp-server
```

El binario se instala en: `~/.cargo/bin/mcp-server`.

### Configuración en Claude Desktop

Agregar en: `~/Library/Application Support/Claude/claude_desktop_config.json` (O crear el archivo si no existe)

Ejemplo:

```json
{
  "mcpServers": {
    "code-analyzer": {
      "command": "/Users/YOUR_USERNAME/.cargo/bin/mcp-server"
    }
  }
}
```

Reemplazar `YOUR_USERNAME` por el usuario correspondiente (correr `whoami` para verificar) y reiniciar Claude Desktop.

### Uso en Claude

El servidor expone las siguientes herramientas:

| Herramienta | Función | Parámetro |
|---|---|---|
| `find_unused_functions` | Detecta funciones definidas pero nunca llamadas | `workspace_path` — path absoluto al proyecto |
| `find_antipatterns` | Detecta long functions, long params, duplicate names y god classes | `workspace_path` |
| `analyze_workspace` | Ejecuta ambos análisis combinados | `workspace_path` |

Ejemplo de prompt en Claude: *"Use find_unused_functions on /Users/me/myproject and tell me what to clean up."*

### Verificar funcionamiento del servidor (opcional)

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}\n{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' \
  | ~/.cargo/bin/mcp-server
```

Debería retornar una respuesta JSON listando las tres herramientas MCP.

---

## Uso como CLI

### Ejecutar sobre un proyecto

```bash
cargo run -- <comando> <directorio-del-proyecto>
```

### Comandos

### `unused` — Funciones sin usar
 
Detecta funciones que están definidas en el proyecto pero nunca son llamadas desde ningún archivo.
 
```bash
cargo run -- unused my-project/
```
 
**Ejemplo de salida:**
```
Unused functions detected:
  - volume()        main.py:12
  - hypotenuse()    main.py:8
```
 
Retorna con código `1` si encuentra funciones sin usar, `0` si no. Útil para CI.
 
### `antipatterns` — Detección de antipatrones
 
Analiza el proyecto en busca de antipatrones de diseño. Detecta los siguientes:
 
#### `[LONG FUNCTION]`
Funciones cuya longitud supera el umbral configurado (por defecto 30 líneas). La longitud se mide desde la línea de definición hasta la línea de cierre, incluyendo líneas en blanco y comentarios.
 
#### `[LONG PARAMS]`
Funciones con más parámetros de los permitidos (por defecto 5, excluyendo `self` en Python). Indica que una función probablemente hace demasiado o necesita una estructura de datos.
 
#### `[DUPLICATE NAME]`
Funciones con el mismo nombre definidas en múltiples archivos del proyecto. Puede causar confusión sobre cuál se está llamando e indica falta de abstracción. Se ignoran automáticamente nombres comunes como `__init__`, `constructor`, `toString`, etc.
 
#### `[GOD CLASS]`
Clases que probablemente violan el principio de responsabilidad única. La detección usa un sistema de puntuación ponderado basado en:
- Cantidad de métodos
- Líneas totales de código en todos los métodos
- Imports distintos usados en los métodos 
- Nombre de la clase — si contiene palabras como `Manager`, `Handler`, `Controller`, etc.

Se reporta si el score supera el umbral configurado.

 
```bash
cargo run -- antipatterns my-project/
```
 
**Ejemplo de salida:**
```
[LONG FUNCTION]    compute_everything   defined in: main.py                 (45 lines)
[LONG PARAMS]      process              defined in: utils.py                (7 parameters)
[DUPLICATE NAME]   serialize_packet     defined in: node.py, protocol.py
[GOD CLASS]        Coordinator          defined in: server.py               (score: 0.70, methods: 9, imports: 2, lines: 87)
```
 
Retorna con código `1` si encuentra antipatrones, `0` si no encuentra ninguno.

### `all` — Todos los análisis
 
Ejecuta `unused` y `antipatterns` en secuencia sobre el mismo proyecto.
 
```bash
cargo run -- all my-project/
```

## Configuración
 
El CLI busca automáticamente un archivo `Config.toml` en el directorio actual. Si no lo encuentra, usa valores por defecto. También se puede especificar un path explícito:
 
```bash
cargo run -- --config New-config.toml antipatterns my-project/
```

**Nota:** El MCP server utiliza únicamente los valores default hardcodeados y no lee Config.toml.

## Uso en CI/CD
 
Todos los comandos retornan código de salida `1` cuando encuentran problemas y `0` cuando el proyecto está limpio. Esto permite integrarlos en pipelines de CI:
 
```yaml
- name: Check for antipatterns
  run: cargo run --release -- antipatterns ./src
 
- name: Check for unused functions
  run: cargo run --release -- unused ./src
```

---

## Estructura del proyecto
 
```
cli-server/
├── src/
│   ├── main.rs                            # Punto de entrada, parsing de argumentos con clap
│   ├── config.rs                          # Structs de configuración, carga desde TOML
│   ├── analysis/
│   │   ├── mod.rs                         # analyze_project() — recorre el directorio y parsea archivos
│   │   ├── unused.rs                      # Detección de funciones sin usar
│   │   └── antipatterns/                  # Detección de antipatrones
│   │       ├── mod.rs                     
│   │       ├── long_function.rs
│   │       ├── long_params.rs
│   │       ├── duplicate_functions.rs
│   │       └── god_class.rs
│   └── bin/                        
│       └── mcp_server.rs                  # Implementación del servidor MCP
├── Config.toml                            # Configuración por defecto (opcional)
└── Cargo.toml
```
