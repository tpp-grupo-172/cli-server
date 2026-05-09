# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                                    # compile
cargo build --release                          # optimized binary
cargo run -- unused <path>                     # detect unused functions
cargo run -- antipatterns <path>               # detect design antipatterns
cargo run -- all <path>                        # both analyses
cargo run -- --config MyConfig.toml unused .  # explicit config path
```

No test suite is configured in this crate. The `tree-sitter-test` sibling repo (`../tree-sitter-test`) must be present for the build to succeed — it is referenced as a local path dependency.

## Architecture

```
main.rs  (clap CLI)
  └─ analysis::unused::run(path)
  └─ analysis::antipatterns::run(path, config)
       │
       ├─ analysis::analyze_project(path)           # scans .py/.ts/.tsx/.js/.jsx, calls run_analysis()
       │    └─ tree_sitter_test::run_analysis()     # returns JSON per file
       │
       ├─ unused.rs
       │    save_function_reference() → Vec<Connections>   # cross-file call graph
       │    save_functions()          → Vec<FunctionsInFiles>
       │    find_unused_functions()   → filter: not in any Connection
       │
       └─ antipatterns/
            long_function.rs     # end_line - line > max_lines
            long_params.rs       # param count (excluding "self") > max_params
            duplicate_functions.rs  # same name in ≥2 files (with ignorelist)
            god_class.rs         # weighted score: methods + imports + lines + name
```

### Call graph resolution (`unused.rs`)

`save_function_reference` builds cross-file edges in three passes per call site:

1. **Direct import** (`import_name` set) — looks up the import in the file's import map.
2. **Object method call** (`object_name` set) — follows `local_variables[obj].assigned_from` → finds the constructor call → resolves its `return_type` → finds the file defining that class.
3. **Same-file call** (no import, no object) — records a self-edge if the function is defined in the same file.

Unused detection filters out `main` and anything starting with `_`.

### God class scoring

Score = `weight_method_count * (methods / method_count_norm).min(1)  +  weight_distinct_imports * ...  +  weight_total_lines * ...  +  weight_name * (1 if name contains a god word else 0)`. Flagged when `score >= flag_threshold`.

## Configuration

The CLI loads `Config.toml` from the current working directory, or the path given to `--config`. Falls back to hardcoded defaults if absent. The checked-in `Config.toml` at the repo root contains the project defaults.

```toml
[long_function]
max_lines = 30          # lines between def and end (inclusive of blanks)

[long_params]
max_params = 5          # excludes "self"

[god_class]
flag_threshold = 0.5
# ... normalization values and weights

[duplicate_functions]
ignored_names = ["__init__", "constructor", ...]
```

## Exit codes

`antipatterns` and `all` exit `1` when violations are found, `0` when clean — useful for CI. `unused` always exits `0` (prints but does not fail).

## Known debug output

`long_function.rs:37` has a stray `println!("{} {}", name, length)` that fires for every function even when no violation is found.
