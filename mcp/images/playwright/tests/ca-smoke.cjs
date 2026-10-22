// Synthetic mkcert layout outside the project; TLS itself is never mocked.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const https = require('node:https');
const http = require('node:http');
const { spawn, execFileSync } = require('node:child_process');
const { createInterface } = require('node:readline');
const { pathToFileURL } = require('node:url');
const project = process.env.SMOKE_CA_PROJECT;
const fixtures = process.env.SMOKE_CA_FIXTURES;
assert.ok(project?.startsWith('/') && fixtures?.startsWith('/'));
const sentinel = 'T04_CA_PRIVATE_KEY_MUST_NOT_BE_READ';
const username = 'synthetic-t04-user';
const password = 'synthetic-t04-secret';
let child;
let stderr = '';
let stdout = '';
let nextId = 0;
const pending = new Map();
const servers = [];
const send = message => child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', ...message })}\n`);
function request(method, params) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timeout: ${stderr}`)); }, 30000);
    pending.set(id, { resolve: value => { clearTimeout(timer); resolve(value); }, reject: error => { clearTimeout(timer); reject(error); } });
    send({ id, method, params });
  });
}
async function tool(name, args) {
  const result = await request('tools/call', { name, arguments: args });
  assert.ok(!result.isError, JSON.stringify(result));
  return result;
}
function evaluated(result) {
  const text = result.content.filter(item => item.type === 'text').map(item => item.text).join('\n');
  const match = text.match(/### Result\s*\n([\s\S]*?)(?=\n### |$)/);
  assert.ok(match, `Missing evaluation result: ${text}`);
  return JSON.parse(match[1]);
}
async function start(caroot) {
  const environment = { ...process.env, PWD: '/', CAROOT: caroot };
  delete environment.AI_PROFILE_PLAYWRIGHT_CA_CERT;
  child = spawn(`${project}/.ai-profiles/opencode/ai-profile-mcp`, ['playwright'], { cwd: '/', env: environment, stdio: ['pipe', 'pipe', 'pipe'] });
  child.stderr.on('data', chunk => { stderr += chunk; });
  child.stdout.on('data', chunk => { stdout += chunk; });
  const fail = error => { for (const waiter of pending.values()) waiter.reject(error); pending.clear(); };
  child.on('error', fail);
  child.stdin.on('error', fail);
  child.on('exit', code => fail(new Error(`MCP exited ${code}: ${stderr}`)));
  createInterface({ input: child.stdout }).on('line', line => {
    try {
      const message = JSON.parse(line);
      if (message.method === 'roots/list') send({ id: message.id, result: { roots: [{ uri: pathToFileURL(project).href }] } });
      else if (message.method && message.id !== undefined) send({ id: message.id, error: { code: -32601, message: 'Method not found' } });
      else if (!message.method) {
        const waiter = pending.get(message.id);
        pending.delete(message.id);
        if (message.error) waiter?.reject(new Error(JSON.stringify(message.error)));
        else waiter?.resolve(message.result);
      }
    } catch (error) { fail(error); }
  });
  await request('initialize', { protocolVersion: '2024-11-05', capabilities: { roots: {} }, clientInfo: { name: 'ca-smoke', version: '1' } });
  send({ method: 'notifications/initialized' });
  await request('tools/list', {});
}
async function stop() {
  if (!child || child.exitCode !== null) return;
  const exited = new Promise(resolve => child.once('exit', resolve));
  child.stdin.end(); child.kill('SIGTERM');
  let timer;
  try { await Promise.race([exited, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('MCP shutdown timeout')), 10000); })]); }
  finally { clearTimeout(timer); }
}
async function listen(server) {
  servers.push(server);
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  return `${server instanceof https.Server ? 'https' : 'http'}://127.0.0.1:${server.address().port}`;
}
function assertNoLeak() {
  for (const value of [stdout, stderr, fs.readFileSync(`${project}/.ai-profiles/state.json`, 'utf8')]) {
    for (const secret of [sentinel, username, password]) assert.ok(!value.includes(secret), 'private data leaked to protocol/log/state');
  }
  for (const entry of fs.readdirSync(`${project}/reports`, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    const bytes = fs.readFileSync(`${project}/reports/${entry.name}`);
    for (const secret of [sentinel, username, password]) assert.ok(!bytes.includes(Buffer.from(secret)), 'private data leaked to report');
  }
}
async function main() {
  const caroot = `${fixtures}/CA with spaces`;
  const privateDir = `${project}/.private`;
  fs.mkdirSync(privateDir, { mode: 0o700 });
  const tls = directory => ({ key: fs.readFileSync(`${directory}/server.key`), cert: fs.readFileSync(`${directory}/server.crt`) });
  const good = await listen(https.createServer(tls(caroot), (req, res) => {
    const auth = `Basic ${Buffer.from(`${username}:${password}`).toString('base64')}`;
    if (req.headers.authorization !== auth) {
      res.writeHead(401, { 'WWW-Authenticate': 'Basic realm="synthetic-t04"' }); res.end('AUTH_REQUIRED'); return;
    }
    res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<h1>AUTO_CA_AUTH_OK</h1>');
  }));
  const other = await listen(https.createServer(tls(`${fixtures}/other CA`), (_req, res) => { res.end('UNTRUSTED_CA'); }));
  fs.copyFileSync(`${caroot}/auth.json`, `${privateDir}/storage.json`);
  fs.chmodSync(`${privateDir}/storage.json`, 0o600);
  fs.writeFileSync(`${privateDir}/playwright-basic-auth.json`, JSON.stringify({ username, password, origin: good }), { mode: 0o600 });
  // No manual CA setting or COMPOSE_FILE; storage state is still an explicit input.
  fs.writeFileSync(`${project}/.ai-profiles/opencode/local.json`, JSON.stringify({ mcp: { playwright: { environment: { AI_PROFILE_PLAYWRIGHT_STORAGE_STATE: '.private/storage.json' } } } }));
  await start(caroot);
  await tool('browser_navigate', { url: good });
  const content = await tool('browser_evaluate', { function: "() => ({text:document.body.innerText,cookie:document.cookie.includes('smoke-session=synthetic-test-only'),axe:window.axe.version})" });
  assert.deepEqual(evaluated(content), { text: 'AUTO_CA_AUTH_OK', cookie: true, axe: '4.14.0' });
  const docker = (...args) => execFileSync('docker', args, { encoding: 'utf8' });
  const ids = docker('ps', '-q', '--filter', 'label=com.docker.compose.service=playwright').trim().split(/\s+/).filter(Boolean);
  const containers = ids.map(id => JSON.parse(docker('inspect', id))[0]).filter(container => container.Config.WorkingDir === project);
  assert.equal(containers.length, 1);
  const mounts = containers[0].Mounts;
  const caMount = mounts.find(mount => mount.Destination === '/run/ai-profile-ca.crt');
  assert.ok(caMount, 'autoimport must mount the public certificate');
  assert.equal(caMount.Source, `${caroot}/rootCA.pem`);
  assert.equal(caMount.RW, false);
  assert.ok(mounts.every(mount => mount.Source !== caroot && !mount.Source.includes('rootCA-key.pem')));
  await tool('browser_take_screenshot', { filename: 'reports/auto-ca.png', type: 'png' });
  const rejected = await request('tools/call', { name: 'browser_navigate', arguments: { url: other } });
  assert.equal(rejected.isError, true);
  assert.match(JSON.stringify(rejected), /ERR_CERT_AUTHORITY_INVALID/);
  // End this context after its TLS-error page; later scenarios get a fresh browser.
  await stop();
  // Missing automatic CA must not block ordinary non-private use.
  const publicUrl = await listen(http.createServer((_req, res) => { res.end('NO_PRIVATE_CA_NEEDED'); }));
  await start(`${fixtures}/missing CA`);
  await tool('browser_navigate', { url: publicUrl });
  assert.match(JSON.stringify(await tool('browser_evaluate', { function: '() => document.body.innerText' })), /NO_PRIVATE_CA_NEEDED/);
  await stop();
  fs.mkdirSync(`${fixtures}/invalid CA`);
  fs.writeFileSync(`${fixtures}/invalid CA/rootCA.pem`, 'not a certificate');
  await assert.rejects(start(`${fixtures}/invalid CA`), /configured CA.*invalid/i);
  assertNoLeak();
  console.log('PASS: automatic CAROOT HTTPS, foreign CA rejected, CA + Basic Auth + storage state, public-file-only RO mount, no key/credential leaks, missing and malformed CA');
}
main().catch(error => { console.error(error.stack); process.exitCode = 1; }).finally(() => {
  if (child && child.exitCode === null) child.kill('SIGKILL');
  for (const server of servers) { server.closeAllConnections(); server.close(); }
});
