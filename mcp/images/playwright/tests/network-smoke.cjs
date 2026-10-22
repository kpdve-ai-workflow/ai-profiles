// Installed launcher + real Chromium; Docker is the VM-local test engine.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const { spawn, execFileSync } = require('node:child_process');
const { createInterface } = require('node:readline');
const { pathToFileURL } = require('node:url');
const project = process.env.SMOKE_NETWORK_PROJECT;
const dnsName = process.env.SMOKE_DNS_CONTAINER;
assert.ok(project?.startsWith('/'));
const docker = (...args) => execFileSync('docker', args, { encoding: 'utf8' });
let child;
let stderr = '';
let nextId = 0;
const pending = new Map();
const server = http.createServer((_req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/html' });
  res.end('<h1>SYNTHETIC_HOST_LOOPBACK_T03</h1>');
});
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
async function start() {
  child = spawn(`${project}/.ai-profiles/opencode/ai-profile-mcp`, ['playwright'], { cwd: '/', env: { ...process.env, PWD: '/' }, stdio: ['pipe', 'pipe', 'pipe'] });
  child.stderr.on('data', chunk => { stderr += chunk; });
  const fail = error => { for (const waiter of pending.values()) waiter.reject(error); pending.clear(); };
  child.on('error', fail);
  child.stdin.on('error', fail);
  child.on('exit', code => fail(new Error(`MCP exited ${code}: ${stderr}`)));
  createInterface({ input: child.stdout }).on('line', line => {
    try {
      const message = JSON.parse(line);
      if (message.method === 'roots/list') {
        send({ id: message.id, result: { roots: [{ uri: pathToFileURL(project).href }] } });
      } else if (message.method && message.id !== undefined) {
        send({ id: message.id, error: { code: -32601, message: 'Method not found' } });
      } else if (!message.method) {
        const waiter = pending.get(message.id);
        pending.delete(message.id);
        if (message.error) waiter?.reject(new Error(JSON.stringify(message.error)));
        else waiter?.resolve(message.result);
      }
    } catch (error) { fail(error); }
  });
  await request('initialize', { protocolVersion: '2024-11-05', capabilities: { roots: {} }, clientInfo: { name: 'network-smoke', version: '1' } });
  send({ method: 'notifications/initialized' });
  await request('tools/list', {});
}
async function stop() {
  if (!child || child.exitCode !== null) return;
  const exited = new Promise(resolve => child.once('exit', resolve));
  child.stdin.end();
  child.kill('SIGTERM');
  let timer;
  try { await Promise.race([exited, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('MCP shutdown timeout')), 10000); })]); }
  finally { clearTimeout(timer); }
}
async function checkPage(url) {
  await tool('browser_navigate', { url });
  const result = await tool('browser_evaluate', { function: '() => document.body.innerText' });
  assert.match(JSON.stringify(result), /SYNTHETIC_HOST_LOOPBACK_T03/);
}
async function main() {
  const payload = `${project}/.ai-profiles/opencode/mcp/playwright`;
  const local = `${project}/.ai-profiles/opencode/local.json`;
  assert.ok(!fs.existsSync(local));
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  await start();
  await checkPage(`http://127.0.0.1:${server.address().port}/`);
  const ids = docker('ps', '-q', '--filter', 'label=com.docker.compose.service=playwright').trim().split(/\s+/).filter(Boolean);
  const containers = ids.map(id => JSON.parse(docker('inspect', id))[0]).filter(container => container.Config.WorkingDir === project);
  assert.equal(containers.length, 1);
  const container = containers[0];
  assert.equal(container.HostConfig.NetworkMode, 'host');
  for (const file of ['/etc/hosts', '/etc/resolv.conf']) {
    const mount = container.Mounts.find(mount => mount.Destination === file);
    assert.ok(mount, `missing host mount ${file}`);
    assert.equal(mount.Type, 'bind');
    assert.equal(mount.Source, file);
    assert.equal(mount.RW, false);
    assert.equal(docker('exec', container.Id, 'cat', file), fs.readFileSync(file, 'utf8'));
  }
  const rejected = await request('tools/call', { name: 'browser_take_screenshot', arguments: { filename: 'root-network.png', type: 'png' } });
  assert.equal(rejected.isError, true);
  assert.ok(!fs.existsSync(`${project}/root-network.png`));
  await stop();
  // Test-only override must win over the automatically prepended host overlay.
  fs.writeFileSync(`${payload}/synthetic-resolv.conf`, 'nameserver 127.203.77.53\noptions timeout:1 attempts:1\n');
  fs.writeFileSync(`${payload}/dns.yaml`, JSON.stringify({ services: { playwright: { volumes: [{ type: 'bind', source: `${payload}/synthetic-resolv.conf`, target: '/etc/resolv.conf', read_only: true, bind: { create_host_path: false } }] } } }));
  fs.writeFileSync(local, JSON.stringify({ mcp: { playwright: { environment: { COMPOSE_FILE: `${payload}/compose.yaml:${payload}/dns.yaml` } } } }));
  await start();
  await checkPage(`http://app.corp.test:${server.address().port}/`);
  assert.match(docker('logs', dnsName), /QUERY app\.corp\.test 1/);
  await tool('browser_take_screenshot', { filename: 'reports/private-dns.png', type: 'png' });
  await stop();
  console.log('PASS: synthetic resolver + host networking, installed launcher from /, loopback HTTP, real hosts/resolver RO mounts, private DNS in Chromium, user overlay precedence');
}
main().catch(error => { console.error(error.stack); process.exitCode = 1; }).finally(() => {
  if (child && child.exitCode === null) child.kill('SIGKILL');
  server.closeAllConnections(); server.close();
});
