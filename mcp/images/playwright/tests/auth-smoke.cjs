// Real HTTP Basic Auth and origin isolation through the installed adapter/launcher.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const { spawn } = require('node:child_process');
const { createInterface } = require('node:readline');

const project = process.env.SMOKE_AUTH_PROJECT;
const adapter = process.env.SMOKE_AUTH_ADAPTER;
assert.ok(project?.startsWith('/'));
assert.ok(adapter?.startsWith('/'));
const username = 'synthetic-auth-user';
const password = 'synthetic-auth-secret';
const wrongPassword = 'synthetic-wrong-secret';
const pending = new Map();
let nextId = 0;
let child;
let stderr = '';
let authServer;
let otherServer;
let publicServer;
const authRequests = [];
const otherRequests = [];

function send(message) {
  child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', ...message })}\n`);
}

function request(method, params) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} timed out`));
    }, 30_000);
    pending.set(id, {
      resolve(value) { clearTimeout(timer); resolve(value); },
      reject(error) { clearTimeout(timer); reject(error); },
    });
    send({ id, method, params });
  });
}

async function tool(name, args) {
  const result = await request('tools/call', { name, arguments: args });
  assert.ok(!result.isError, `${name} failed: ${JSON.stringify(result)}`);
  return result;
}

async function expectUnauthorizedNavigation(url) {
  const result = await request('tools/call', {
    name: 'browser_navigate', arguments: { url },
  });
  assert.equal(result.isError, true, 'an HTTP 401 must not be reported as a successful navigation');
  assert.match(JSON.stringify(result), /ERR_INVALID_AUTH_CREDENTIALS/);
}

function evaluated(result) {
  const text = result.content.filter(item => item.type === 'text').map(item => item.text).join('\n');
  const match = text.match(/### Result\s*\n([\s\S]*?)(?=\n### |$)/);
  assert.ok(match, `Missing evaluation result: ${text}`);
  return JSON.parse(match[1]);
}

async function listen(server) {
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  return `http://127.0.0.1:${server.address().port}`;
}

async function startMcp() {
  console.error('auth-smoke: starting MCP');
  child = spawn(adapter, ['playwright'], {
    cwd: '/',
    env: { ...process.env, PWD: '/', AI_PROFILE_PROJECT_DIR: '/wrong/inherited' },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  const exited = new Promise(resolve => child.once('exit', resolve));
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => { stderr += chunk; });
  const failPending = error => {
    for (const waiter of pending.values()) waiter.reject(error);
    pending.clear();
  };
  child.on('error', failPending);
  child.stdin.on('error', failPending);
  child.on('exit', (code, signal) => failPending(new Error(`MCP exited: ${code ?? signal}`)));
  createInterface({ input: child.stdout }).on('line', line => {
    try {
      const message = JSON.parse(line);
      if (message.method) {
        if (message.method === 'roots/list' && message.id !== undefined) {
          send({ id: message.id, result: { roots: [{ uri: require('node:url').pathToFileURL(project).href }] } });
        } else if (message.id !== undefined) {
          send({ id: message.id, error: { code: -32601, message: 'Method not found' } });
        }
        return;
      }
      const waiter = pending.get(message.id);
      if (!waiter) return;
      pending.delete(message.id);
      if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
      else waiter.resolve(message.result);
    } catch (error) {
      failPending(error);
    }
  });
  await request('initialize', {
    protocolVersion: '2024-11-05', capabilities: { roots: { listChanged: true } },
    clientInfo: { name: 'playwright-basic-auth-smoke', version: '1.0.0' },
  });
  send({ method: 'notifications/initialized' });
  const discovery = await request('tools/list', {});
  console.error('auth-smoke: MCP initialized');
  assert.ok(!JSON.stringify(discovery).includes(username));
  assert.ok(!JSON.stringify(discovery).includes(password));
  return { exited };
}

async function stopMcp(exited) {
  if (!child || child.exitCode !== null) return;
  child.stdin.end();
  child.kill('SIGTERM');
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('MCP did not stop')), 10_000);
    exited.then(
      status => { clearTimeout(timer); resolve(status); },
      error => { clearTimeout(timer); reject(error); },
    );
  });
}

function writeCredentials(origin, secret) {
  const directory = `${project}/.ai-profiles/.private`;
  fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
  fs.chmodSync(directory, 0o700);
  fs.writeFileSync(`${directory}/playwright-basic-auth.json`, JSON.stringify({
    username, password: secret, origin,
  }), { mode: 0o600 });
  fs.chmodSync(`${directory}/playwright-basic-auth.json`, 0o600);
}

async function main() {
  assert.ok(!fs.existsSync(`${project}/.ai-profiles/reports`), 'fixture must not pre-create reports');
  assert.ok(!fs.existsSync(`${project}/.ai-profiles/opencode/local.json`), 'fixture must not use local.json');
  assert.ok(!fs.existsSync(`${project}/.ai-profiles/.private/playwright-basic-auth.json`), 'fixture starts without credentials');
  assert.ok(fs.existsSync(`${project}/.ai-profiles/opencode/mcp/playwright/playwright-launcher`));

  authServer = http.createServer((req, res) => {
    const observed = { url: req.url, authorization: req.headers.authorization };
    authRequests.push(observed);
    if (req.url === '/redirect') {
      observed.status = 302;
      res.writeHead(302, { Location: `${otherOrigin}/redirected` });
      res.end();
      return;
    }
    const expected = `Basic ${Buffer.from(`${username}:${password}`).toString('base64')}`;
    if (req.headers.authorization === expected) {
      observed.status = 200;
      res.writeHead(200, { 'Content-Type': 'text/plain' });
      res.end('AUTH_OK');
    } else {
      observed.status = 401;
      res.writeHead(401, { 'WWW-Authenticate': 'Basic realm="synthetic"', 'Content-Type': 'text/plain' });
      res.end('AUTH_REQUIRED');
    }
  });
  otherServer = http.createServer((req, res) => {
    otherRequests.push({ url: req.url, authorization: req.headers.authorization });
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end('OTHER_OK');
  });
  publicServer = http.createServer((_req, res) => {
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end('PUBLIC_OK');
  });
  const otherOrigin = await listen(otherServer);
  const authOrigin = await listen(authServer);
  const publicOrigin = await listen(publicServer);

  let { exited } = await startMcp();
  console.error('auth-smoke: reports prepared');
  assert.ok(fs.statSync(`${project}/.ai-profiles/reports`).isDirectory(), 'launcher must create reports before Compose');
  await tool('browser_navigate', { url: publicOrigin });
  console.error('auth-smoke: public navigation complete');
  assert.equal(evaluated(await tool('browser_evaluate', { function: '() => document.body.innerText' })), 'PUBLIC_OK');
  await expectUnauthorizedNavigation(authOrigin);
  console.error('auth-smoke: unauthenticated navigation complete');
  assert.ok(authRequests.some(request => request.url === '/' && request.status === 401 && request.authorization === undefined),
    'no credentials should be sent before setup');
  await stopMcp(exited);

  writeCredentials(authOrigin, wrongPassword);
  ({ exited } = await startMcp());
  console.error('auth-smoke: wrong-password MCP initialized');
  await request('tools/call', { name: 'browser_navigate', arguments: { url: authOrigin } });
  console.error('auth-smoke: wrong-password navigation complete');
  assert.ok(authRequests.some(request => request.url === '/' && request.status === 401 &&
    request.authorization === `Basic ${Buffer.from(`${username}:${wrongPassword}`).toString('base64')}`),
  'a wrong password must receive the real HTTP 401 response');
  await stopMcp(exited);

  const wrongPort = new URL(authOrigin);
  const currentPort = Number(wrongPort.port);
  wrongPort.port = String(currentPort === 65535 ? currentPort - 1 : currentPort + 1);
  writeCredentials(wrongPort.origin, password);
  ({ exited } = await startMcp());
  await expectUnauthorizedNavigation(authOrigin);
  assert.ok(authRequests.some(request => request.url === '/' && request.status === 401 &&
    request.authorization === undefined), 'credentials for another port must not authenticate');
  await stopMcp(exited);

  writeCredentials(authOrigin, password);
  ({ exited } = await startMcp());
  console.error('auth-smoke: valid-password MCP initialized');
  await tool('browser_navigate', { url: authOrigin });
  console.error('auth-smoke: authenticated navigation complete');
  assert.equal(evaluated(await tool('browser_evaluate', { function: '() => document.body.innerText' })), 'AUTH_OK');
  assert.ok(authRequests.some(request => request.authorization ===
    `Basic ${Buffer.from(`${username}:${password}`).toString('base64')}`));

  await tool('browser_navigate', { url: `${authOrigin}/redirect` });
  assert.ok(otherRequests.some(request => request.url === '/redirected'));
  assert.ok(otherRequests.every(request => request.authorization === undefined),
    'Authorization must not cross to another origin, including redirects');
  await tool('browser_take_screenshot', { filename: '.ai-profiles/reports/auth.png', type: 'png' });
  for (const entry of fs.readdirSync(`${project}/.ai-profiles/reports`, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    const contents = fs.readFileSync(`${project}/.ai-profiles/reports/${entry.name}`);
    assert.ok(!contents.includes(Buffer.from(username)), 'reports must not contain the username');
    assert.ok(!contents.includes(Buffer.from(password)), 'reports must not contain the password');
  }
  const state = fs.readFileSync(`${project}/.ai-profiles/state.json`, 'utf8');
  assert.ok(!state.includes(username));
  assert.ok(!state.includes(password));
  assert.ok(!stderr.includes(username));
  assert.ok(!stderr.includes(password));
  await stopMcp(exited);
  console.log('PASS: clean installed launcher, automatic reports, optional HTTP Basic Auth, 401/200, origin and redirect isolation');
}

main().catch(error => {
  console.error(`FAIL: ${error.stack}`);
  process.exitCode = 1;
}).finally(() => {
  if (child && child.exitCode === null) child.kill('SIGKILL');
  for (const server of [authServer, otherServer, publicServer]) {
    if (server) { server.closeAllConnections(); server.close(); }
  }
});
