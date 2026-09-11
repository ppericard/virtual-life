// Discover source and test files so new modules cannot silently miss syntax checks.
import {readdirSync} from 'node:fs';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';

function files(directory) {
  return readdirSync(directory, {withFileTypes:true}).flatMap(entry => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? files(path) : /\.m?js$/.test(path) ? [path] : [];
  });
}

function run(args) {
  const result = spawnSync(process.execPath, args, {stdio:'inherit'});
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const source = ['playwright.config.js', ...files('web'), ...files('tests'), ...files('scripts')];
for (const path of source.sort()) run(['--check', path]);
run(['--test', ...source.filter(path => path.endsWith('.test.js')).sort()]);
