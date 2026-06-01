#!/usr/bin/env node
'use strict';

/**
 * Fallback de inferencia de tipos usando la TypeScript Compiler API.
 * Llamado desde el LSP backend en Rust cuando el análisis estático no puede
 * resolver el tipo de una variable.
 *
 * Uso:    node ts_analyzer.js <file.ts>
 * Salida: JSON { "varname@line": { "type": "ClassName", "module_path": "/abs/path.ts" } }
 */

const path = require('path');
const fs = require('fs');

const SKIP_NAMES = new Set([
  'this', 'self', 'console', 'process', 'require', 'module', 'exports',
  'window', 'document', 'Math', 'JSON', 'Object', 'Array', 'String',
  'Number', 'Boolean', 'Promise', 'Error', 'Symbol', 'Map', 'Set',
  'WeakMap', 'WeakSet', 'Date', 'RegExp', 'undefined', 'null',
  'setTimeout', 'setInterval', 'clearTimeout', 'clearInterval',
]);

const BUILTIN_TYPES = new Set([
  'string', 'number', 'boolean', 'void', 'any', 'unknown', 'never',
  'object', 'symbol', 'bigint', 'null', 'undefined', 'Function',
  'Array', 'Promise', 'Error', 'Object',
  '__type', // tipo anónimo inline, no es una clase resoluble
]);

function findTypescript(startDir) {
  let dir = startDir;
  while (true) {
    const candidate = path.join(dir, 'node_modules', 'typescript');
    if (fs.existsSync(path.join(candidate, 'package.json'))) return candidate;
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

function findTsConfig(startDir) {
  let dir = startDir;
  while (true) {
    const candidate = path.join(dir, 'tsconfig.json');
    if (fs.existsSync(candidate)) return candidate;
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

function analyze(filePath) {
  const absPath = path.resolve(filePath);
  const fileDir = path.dirname(absPath);

  const tsPath = findTypescript(fileDir);
  if (!tsPath) return {};

  let ts;
  try { ts = require(tsPath); } catch (e) { return {}; }

  let compilerOptions = {
    allowJs: true,
    checkJs: true,
    noEmit: true,
    strict: false,
    skipLibCheck: true,
    moduleResolution: ts.ModuleResolutionKind.NodeJs,
    target: ts.ScriptTarget.ES2020,
  };

  const tsconfigPath = findTsConfig(fileDir);
  if (tsconfigPath) {
    try {
      const configFile = ts.readConfigFile(tsconfigPath, ts.sys.readFile);
      if (!configFile.error) {
        const parsed = ts.parseJsonConfigFileContent(
          configFile.config, ts.sys, path.dirname(tsconfigPath)
        );
        compilerOptions = { ...parsed.options, noEmit: true, skipLibCheck: true };
      }
    } catch (e) {}
  }

  let program;
  try {
    program = ts.createProgram([absPath], compilerOptions);
  } catch (e) { return {}; }

  const checker = program.getTypeChecker();
  const sourceFile = program.getSourceFile(absPath);
  if (!sourceFile) return {};

  const results = {};

  function visit(node) {
    if (ts.isPropertyAccessExpression(node)) {
      const expr = node.expression;
      if (ts.isIdentifier(expr)) {
        const name = expr.text;
        if (!SKIP_NAMES.has(name)) {
          const line = sourceFile.getLineAndCharacterOfPosition(expr.getStart()).line + 1;
          const key = `${name}@${line}`;
          if (!results[key]) {
            try {
              const type = checker.getTypeAtLocation(expr);
              const symbol = type.getSymbol() || type.aliasSymbol;
              if (symbol) {
                const decls = symbol.getDeclarations();
                if (decls && decls.length > 0) {
                  const declFile = decls[0].getSourceFile();
                  const declPath = declFile.fileName;
                  if (!declPath.includes('node_modules') && !declPath.includes('/lib.')) {
                    if (!BUILTIN_TYPES.has(symbol.name)) {
                      results[key] = { type: symbol.name, module_path: declPath };
                    }
                  }
                }
              }
            } catch (e) {}
          }
        }
      }
    }
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return results;
}

const filePath = process.argv[2];
if (!filePath) {
  process.stdout.write('{}\n');
  process.exit(0);
}

try {
  process.stdout.write(JSON.stringify(analyze(filePath)) + '\n');
} catch (e) {
  process.stdout.write('{}\n');
}
