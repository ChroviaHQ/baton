import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? '')) {
  console.error('Usage: npm run release -- <major.minor.patch>');
  process.exit(1);
}

const CHINA_DOMAINS_URL =
  'https://raw.githubusercontent.com/ChroviaHQ/domain-list-community/release/cn.txt';
const CHINA_DOMAINS_PATH = 'crates/baton-core/data/china-domains.txt';

const run = (command, args) => execFileSync(command, args, {stdio: 'inherit'});

const updateJson = (path, update) => {
  const json = JSON.parse(readFileSync(path, 'utf8'));
  update(json);
  writeFileSync(path, `${JSON.stringify(json, null, 2)}\n`);
};

if (execFileSync('git', ['status', '--porcelain'], {encoding: 'utf8'}).trim()) {
  console.error('Commit or stash your changes before releasing.');
  process.exit(1);
}

const chinaDomainList = execFileSync('curl', ['-fsSL', CHINA_DOMAINS_URL], {encoding: 'utf8'});
const chinaDomains = new Set(
  Array.from(chinaDomainList.matchAll(/^(?:domain|full):([^:\s]+)/gm), match =>
    match[1].toLowerCase()),
);
if (chinaDomains.size === 0) {
  console.error(`No China domains found at ${CHINA_DOMAINS_URL}.`);
  process.exit(1);
}
writeFileSync(CHINA_DOMAINS_PATH, `${[...chinaDomains].sort().join('\n')}\n`);

updateJson('package.json', json => {
  json.version = version;
});
updateJson('package-lock.json', json => {
  json.version = version;
  json.packages[''].version = version;
});
const cargoToml = readFileSync('Cargo.toml', 'utf8');
writeFileSync('Cargo.toml', cargoToml.replace(/^version = "[^"]+"/m, `version = "${version}"`));
run('cargo', ['update', '--workspace', '--offline']);
run('git', ['commit', '-am', `Release v${version}`]);
run('git', ['tag', `v${version}`]);
run('git', ['push', 'origin', 'HEAD', `v${version}`]);
