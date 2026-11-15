import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const secretPattern = /DB_PASSWORD|BEGIN PRIVATE KEY|ghp_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}/;

function fail(message) {
  console.log(`FAIL ${message}`);
  process.exit(1);
}

const probeDir = mkdtempSync(join(tmpdir(), 'm003-'));
const probePath = join(probeDir, 'probe.html');
writeFileSync(probePath, '<p>DB_PASSWORD=injected</p>');
const probeText = readFileSync(probePath, 'utf8');
rmSync(probeDir, { recursive: true, force: true });
if (!secretPattern.test(probeText)) fail('scanner self-test missed DB_PASSWORD');
console.log('PASS scanner self-test');

function walk(dir, skipGenerated) {
  const files = [];
  for (const name of readdirSync(dir)) {
    if (skipGenerated && (name === 'node_modules' || name === 'dist' || name === '.astro')) continue;
    const path = join(dir, name);
    const info = statSync(path);
    if (info.isDirectory()) files.push(...walk(path, skipGenerated));
    else files.push(path);
  }
  return files;
}

const dist = join(root, 'apps', 'web', 'dist');
try {
  statSync(dist);
} catch {
  console.log('FAIL dist missing');
  process.exit(2);
}

const distHit = walk(dist, false).find((path) => secretPattern.test(readFileSync(path, 'utf8')));
if (distHit) fail('dist contains a server secret or token pattern');
console.log('PASS dist has no DB_PASSWORD or token pattern');

const webRoot = join(root, 'apps', 'web');
const sourceFiles = walk(webRoot, true);
const userPath = /C:\\Users|C:\/Users/;
const pathHit = sourceFiles.find((path) => userPath.test(readFileSync(path, 'utf8')));
if (pathHit) fail('apps/web contains a user profile absolute path');
console.log('PASS apps/web has no user profile absolute path');

const examplePath = join(root, '.env.example');
const example = readFileSync(examplePath, 'utf8');
if (!example.includes('BROWSER_PUBLIC') || !example.includes('SERVER_ONLY')) {
  fail('.env.example does not separate browser and server names');
}
const filled = example.split(/\r?\n/).some((line) => /^\s*(DB_PASSWORD|DB_USER|TOKEN|SECRET)\s*=\s*\S+/.test(line));
if (filled) fail('.env.example contains a filled secret');
console.log('PASS env example classifies names and has no secret value');

const config = readFileSync(join(webRoot, 'vite.config.ts'), 'utf8');
if (!config.includes('127.0.0.1') || !config.includes('4321')) {
  fail('web server host or port is not pinned');
}
console.log('PASS web server is 127.0.0.1:4321');

// 기능: 공개 웹 개발 서버의 API 전달을 검사한다.
// 목적: 탐색 화면의 GET /api/search 는 로컬 Axum 으로만 넘기고, 다른 주소는 막는다.
// 연관: apps/web/vite.config.ts, apps/web/src/scripts/explore.ts, services/api.
const allowedProxy = "proxy: { '/api': 'http://127.0.0.1:8080' }";
const proxyFiles = sourceFiles.filter((path) => /^\s*proxy\s*:/m.test(readFileSync(path, 'utf8')));
const configPath = join(webRoot, 'vite.config.ts');
const proxyLines = (config.match(/^\s*proxy\s*:/gm) ?? []).length;
if (proxyFiles.length === 0) {
  console.log('PASS no public-web API proxy');
} else if (proxyFiles.length !== 1 || proxyFiles[0] !== configPath || proxyLines !== 1 || !config.includes(allowedProxy)) {
  fail('public web API proxy must be only /api to http://127.0.0.1:8080');
} else {
  console.log('PASS public web API proxy is loopback 8080');
}

function present(path) {
  try {
    statSync(path);
    return true;
  } catch {
    return false;
  }
}

console.log(`CMS_PRESENT=${present(join(root, 'services', 'cms'))}`);
console.log(`COMPOSE_PRESENT=${present(join(root, 'infra', 'local', 'compose.yaml'))}`);
console.log(`LOCAL_PROFILE_PRESENT=${present(join(root, 'services', 'cms', 'application-local.yml'))}`);
console.log('PASS env classification');
