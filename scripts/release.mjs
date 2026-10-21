import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? '')) {
  console.error('Usage: npm run release -- <major.minor.patch>');
  process.exit(1);
}

const run = (command, args) => execFileSync(command, args, {stdio: 'inherit'});

if (execFileSync('git', ['status', '--porcelain'], {encoding: 'utf8'}).trim()) {
  console.error('Commit or stash your changes before releasing.');
  process.exit(1);
}

run(process.execPath, [process.env.npm_execpath, 'version', version, '--no-git-tag-version']);
const cargoToml = readFileSync('Cargo.toml', 'utf8');
writeFileSync('Cargo.toml', cargoToml.replace(/^version = "[^"]+"/m, `version = "${version}"`));
run('cargo', ['update', '--workspace', '--offline']);
run('git', ['commit', '-am', `Release v${version}`]);
run('git', ['tag', `v${version}`]);
run('git', ['push', 'origin', 'HEAD', `v${version}`]);
