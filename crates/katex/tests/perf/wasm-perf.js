#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const { performance } = require('node:perf_hooks');
const yaml = require('js-yaml');

const DATA_FILE = path.resolve(__dirname, '../../../../KaTeX/test/screenshotter/ss_data.yaml');
const WASM_PKG = path.resolve(__dirname, '../../pkg/katex.js');

const TESTS_TO_RUN = [
  'AccentsText',
  'ArrayMode',
  'GroupMacros',
  'MathBb',
  'SqrtRoot',
  'StretchyAccent',
  'Units',
];

function parseArgs(argv) {
  const options = {
    cases: [],
    list: false,
    help: false,
  };

  for (let index = 2; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case '--case':
      case '--cases': {
        const value = argv[index + 1];
        if (!value) {
          throw new Error('Missing value for --case.');
        }
        options.cases.push(value);
        index += 1;
        break;
      }
      case '--list':
        options.list = true;
        break;
      case '--help':
      case '-h':
        options.help = true;
        break;
      default:
        throw new Error(`Unknown argument "${arg}". Use --help for usage.`);
    }
  }

  return options;
}

function printHelp() {
  console.log(`KaTeX wasm rendering benchmark\n\
Usage: node wasm-perf.js [options]\n\
\nOptions:\n\
  --case <NAME>    Run a single test case (can be repeated).\n\
  --list           Print the available case names and exit.\n\
  -h, --help       Show this message.`);
}

function resolveCases(requested) {
  if (requested.length === 0) {
    return TESTS_TO_RUN;
  }

  return requested.map((name) => {
    const match = TESTS_TO_RUN.find((candidate) => candidate.toLowerCase() === name.toLowerCase());
    if (!match) {
      throw new Error(`Unknown test case "${name}". Known cases: ${TESTS_TO_RUN.join(', ')}`);
    }
    return match;
  });
}

function loadDataset(selected) {
  const raw = yaml.load(fs.readFileSync(DATA_FILE, 'utf8'));
  return selected.map((name) => {
    const value = raw[name];
    if (!value) {
      throw new Error(`Missing test case "${name}" in ss_data.yaml`);
    }

    if (typeof value === 'string') {
      return { name, tex: value, options: {} };
    }

    const options = {};
    if (value.display !== undefined) {
      options.displayMode = Boolean(value.display);
    }
    if (value.macros) {
      options.macros = value.macros;
    }

    return { name, tex: value.tex, options };
  });
}

function runBenchmark(renderToString, cases) {
  console.log('KaTeX wasm rendering benchmark\n');
  for (const test of cases) {
    runCase(renderToString, test);
  }
}

function runCase(renderToString, test) {
  let checksum = renderToString(test.tex, test.options).length;
  let iterations = 1;
  let elapsed = 0;

  while (elapsed < 250 && iterations < 1_048_576) {
    const start = performance.now();
    for (let i = 0; i < iterations; i += 1) {
      const result = renderToString(test.tex, test.options);
      checksum = (checksum + result.length) >>> 0;
    }
    elapsed = performance.now() - start;
    if (elapsed < 250) {
      iterations *= 2;
    }
  }

  const opsPerSecond = iterations / (elapsed / 1000);
  console.log(
    `${test.name.padEnd(16)} ${opsPerSecond.toFixed(2).padStart(12)} ops/s (${iterations} iterations, checksum ${checksum})`,
  );
}

function main() {
  const options = parseArgs(process.argv);
  if (options.help) {
    printHelp();
    return;
  }
  if (options.list) {
    console.log(TESTS_TO_RUN.join('\n'));
    return;
  }

  if (!fs.existsSync(WASM_PKG)) {
    console.error('Missing wasm bindings. Run `wasm-pack build --release --target nodejs --features wasm`.');
    process.exit(1);
  }

  const { renderToString } = require(WASM_PKG);
  if (typeof renderToString !== 'function') {
    throw new Error('Invalid wasm bindings: renderToString export not found.');
  }

  const selected = resolveCases(options.cases);
  const cases = loadDataset(selected);
  runBenchmark(renderToString, cases);
}

main();
