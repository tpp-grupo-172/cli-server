#!/usr/bin/env python3
"""
Fallback de inferencia de tipos usando jedi.
Llamado desde el LSP backend en Rust cuando el análisis estático no puede
resolver el tipo de una variable.

Uso: python3 jedi_analyzer.py <file.py>
Salida: JSON { "varname@line": { "type": "ClassName", "module_path": "/abs/path.py" } }
"""
import sys
import json
import re

# Identificadores que no tiene sentido inferir
SKIP_NAMES = frozenset({
    'self', 'cls', 'super', 'True', 'False', 'None',
    'print', 'len', 'range', 'type', 'isinstance', 'hasattr',
    'getattr', 'setattr', 'enumerate', 'zip', 'map', 'filter',
    'sorted', 'reversed', 'open', 'input', 'vars', 'dir',
})

# Tipos built-in que no corresponden a clases de usuario
BUILTIN_TYPES = frozenset({
    'str', 'int', 'float', 'bool', 'list', 'dict', 'set', 'tuple',
    'bytes', 'bytearray', 'NoneType', 'module', 'function', 'method',
    'type', 'object', 'Exception', 'BaseException', 'property',
    'classmethod', 'staticmethod', 'generator', 'coroutine',
})


def analyze(file_path: str) -> dict:
    try:
        import jedi
    except ImportError:
        return {}

    try:
        with open(file_path, 'r', encoding='utf-8', errors='replace') as f:
            source = f.read()
    except Exception:
        return {}

    try:
        script = jedi.Script(source, path=file_path)
    except Exception:
        return {}

    lines = source.splitlines()
    results = {}

    for line_idx, line_text in enumerate(lines):
        line_num = line_idx + 1  # jedi usa líneas 1-based

        # Buscar identificadores inmediatamente antes de un punto: name.algo
        # Esto captura tanto variables simples (order.complete())
        # como atributos de instancia (order_service.funcion()) dentro de
        # expresiones como self.order_service.funcion()
        for match in re.finditer(r'(?<![.\w])([a-zA-Z_]\w*)(?=\.)', line_text):
            name = match.group(1)
            if name in SKIP_NAMES:
                continue

            col = match.start(1)  # 0-indexed, jedi también usa 0-based columnas
            key = f"{name}@{line_num}"

            if key in results:
                continue

            try:
                inferred = script.infer(line_num, col)
                if not inferred:
                    continue

                ti = inferred[0]
                class_name = ti.name
                module_path = ti.module_path

                if not class_name or class_name in BUILTIN_TYPES:
                    continue
                if module_path is None:
                    continue

                results[key] = {
                    "type": class_name,
                    "module_path": str(module_path),
                }
            except Exception:
                pass

    return results


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.stdout.write('{}\n')
        sys.exit(0)

    output = analyze(sys.argv[1])
    sys.stdout.write(json.dumps(output) + '\n')
