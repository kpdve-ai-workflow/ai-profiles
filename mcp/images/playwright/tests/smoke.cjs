// No host Node, npm install, or browser-selection overrides: exercise the image as shipped.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const https = require('node:https');
const { spawn } = require('node:child_process');
const { createInterface } = require('node:readline');

const project = process.env.SMOKE_PROJECT_DIR || '/project';
const report = `${project}/reports/wcag`;
const pending = new Map();
let nextId = 0;
let child;
let server;
let tlsServer;
const deadline = setTimeout(() => {
  console.error('FAIL: smoke test exceeded 120 seconds');
  process.exit(1); // Docker tears down all remaining container processes.
}, 120_000);

function send(message) {
  child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', ...message })}\n`);
}

function request(method, params) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} timed out after 30 seconds`));
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

function evaluated(result) {
  const text = result.content.filter(item => item.type === 'text').map(item => item.text).join('\n');
  const match = text.match(/### Result\s*\n([\s\S]*?)(?=\n### |$)/);
  assert.ok(match, `Missing evaluation result: ${text}`);
  return JSON.parse(match[1]);
}

async function main() {
  assert.equal(process.env.HOST_UID, '');
  assert.equal(process.env.HOST_GID, '');
  // An image-level diagnostic override must not silently mask the regression.
  assert.equal(process.env.PLAYWRIGHT_MCP_BROWSER, undefined);
  assert.equal(process.env.PLAYWRIGHT_MCP_EXECUTABLE_PATH, undefined);
  const mounts = fs.readFileSync('/proc/self/mountinfo', 'utf8').split('\n');
  assert.ok(mounts.some(line => line.split(' ')[4] === project && line.split(' ')[5].split(',').includes('ro')));
  assert.ok(mounts.some(line => line.split(' ')[4] === report && line.split(' ')[5].split(',').includes('rw')));

  // This is a fresh test-only report mount. Exercise UID/GID inference, not an override.
  fs.chownSync(report, 1000, 1000);
  child = spawn('/usr/local/bin/playwright-entrypoint', [], { stdio: ['pipe', 'pipe', 'inherit'] });
  const exited = new Promise(resolve => child.once('exit', resolve));
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
      const waiter = pending.get(message.id);
      if (!waiter) return;
      pending.delete(message.id);
      if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
      else waiter.resolve(message.result);
    } catch (error) {
      failPending(error);
    }
  });

  // The entrypoint starts as root to prepare its home, then drops privileges.
  // Also run the fixture server and artifact writer as the same non-root identity.
  process.setgroups([]);
  process.setgid(1000);
  process.setuid(1000);
  assert.equal(process.getuid(), 1000);
  assert.equal(process.getgid(), 1000);

  const initialized = await request('initialize', {
    protocolVersion: '2024-11-05', capabilities: {},
    clientInfo: { name: 'playwright-container-smoke', version: '1.0.0' },
  });
  send({ method: 'notifications/initialized' });
  const discovery = await request('tools/list', {});
  fs.writeFileSync(`${report}/mcp-discovery.json`, JSON.stringify({ initialized, ...discovery }, null, 2));
  for (const [name, fields] of Object.entries({
    browser_navigate: ['url'], browser_evaluate: ['function'], browser_take_screenshot: ['filename'],
  })) {
    const schema = discovery.tools.find(item => item.name === name)?.inputSchema;
    assert.ok(schema, `Missing tool schema: ${name}`);
    for (const field of fields) assert.ok(schema.properties[field], `${name} lacks ${field}`);
  }
  const status = fs.readFileSync(`/proc/${child.pid}/status`, 'utf8');
  for (const field of ['Uid', 'Gid']) {
    assert.deepEqual(status.match(new RegExp(`^${field}:\\s+(.+)$`, 'm'))[1].trim().split(/\s+/), ['1000', '1000', '1000', '1000']);
  }

  const fixture = fs.readFileSync(`${project}/fixture.html`);
  const respond = (req, res) => {
    if (req.url === '/pixel.svg') {
      res.writeHead(200, { 'Content-Type': 'image/svg+xml' });
      res.end('<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>');
    } else {
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      res.end(fixture);
    }
  };
  server = http.createServer(respond);
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const url = `http://127.0.0.1:${server.address().port}/`;
  await tool('browser_navigate', { url });
  assert.equal(evaluated(await tool('browser_evaluate', { function: '() => window.axe.version' })), '4.14.0');
  const raw = await tool('browser_evaluate', { function: 'async () => await window.axe.run(document)' });
  fs.writeFileSync(`${report}/axe-raw.json`, JSON.stringify(raw, null, 2));
  const scan = evaluated(raw);
  fs.writeFileSync(`${report}/axe-scan.json`, JSON.stringify(scan, null, 2));
  assert.equal(scan.testEngine.version, '4.14.0');
  const violations = new Set(scan.violations.map(item => item.id));
  assert.ok(violations.has('button-name'), 'axe missed the unlabeled button');
  assert.ok(violations.has('image-alt'), 'axe missed the image without alt text');
  await tool('browser_take_screenshot', { filename: `${report}/screenshot.png`, type: 'png', scale: 'css' });
  const screenshot = fs.readFileSync(`${report}/screenshot.png`);
  assert.equal(screenshot.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  assert.equal(fs.statSync(`${report}/screenshot.png`).uid, 1000);
  await tool('browser_navigate', { url: `${url}?second-navigation` });
  assert.equal(evaluated(await tool('browser_evaluate', { function: '() => window.axe.version' })), '4.14.0');
  tlsServer = https.createServer({
    key: fs.readFileSync(`${project}/.private/server.key`),
    cert: fs.readFileSync(`${project}/.private/server.crt`),
  }, respond);
  await new Promise((resolve, reject) => {
    tlsServer.once('error', reject);
    tlsServer.listen(0, '127.0.0.1', resolve);
  });
  await tool('browser_navigate', { url: `https://127.0.0.1:${tlsServer.address().port}/` });
  assert.deepEqual(evaluated(await tool('browser_evaluate', {
    function: "() => ({axe:window.axe.version,authenticated:document.cookie.includes('smoke-session=synthetic-test-only')})",
  })), { axe: '4.14.0', authenticated: true });
  console.log(`PASS: non-root MCP, axe 4.14.0, screenshot, repeat navigation, trusted HTTPS, and storage state (${process.platform}/${process.arch})`);
  child.stdin.end();
  child.kill('SIGTERM');
  await Promise.race([exited, new Promise(resolve => setTimeout(resolve, 2000))]);
}

main().catch(error => {
  console.error(`FAIL: ${error.stack}`);
  process.exitCode = 1;
}).finally(() => {
  clearTimeout(deadline);
  for (const waiter of pending.values()) waiter.reject(new Error('Smoke test finished'));
  pending.clear();
  if (child && child.exitCode === null) child.kill('SIGKILL');
  if (server) { server.closeAllConnections(); server.close(); }
  if (tlsServer) { tlsServer.closeAllConnections(); tlsServer.close(); }
});
