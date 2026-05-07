# CLI-Server

Herramienta de línea de comandos para analizar proyectos de código fuente, en busca de funciones sin usar y antipatrones de diseño. 

## Instalación y uso

### Requisitos

- [Rust](https://rust-lang.org/tools/install/)
- Cargo (incluido con Rust)
- El repositorio tree-sitter-test clonado localmente (referenciado por path en Cargo.toml)

### Compilar

```bash
cargo build --release
```

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

## Uso en CI/CD
 
Todos los comandos retornan código de salida `1` cuando encuentran problemas y `0` cuando el proyecto está limpio. Esto permite integrarlos en pipelines de CI:
 
```yaml
- name: Check for antipatterns
  run: cargo run --release -- antipatterns ./src
 
- name: Check for unused functions
  run: cargo run --release -- unused ./src
```

## Estructura del proyecto
 
```
cli-server/
├── src/
│   ├── main.rs                            # Punto de entrada, parsing de argumentos con clap
│   ├── config.rs                          # Structs de configuración, carga desde TOML
│   └── analysis/
│       ├── mod.rs                         # analyze_project() — recorre el directorio y parsea archivos
│       ├── unused.rs                      # Detección de funciones sin usar
│       └── antipatterns/                  # Detección de antipatrones
│           ├── mod.rs                     
│           ├── long_function.rs
│           ├── long_params.rs
│           ├── duplicate_functions.rs
│           └── god_class.rs
├── Config.toml                            # Configuración por defecto (opcional)
└── Cargo.toml
```
