// Create disposable TLS/auth fixtures inside Docker, without host OpenSSL/Node.
const fs = require('node:fs');
const { execFileSync } = require('node:child_process');

const directory = '/project/.private';
fs.mkdirSync(directory, { mode: 0o700 });
function openssl(...args) {
  execFileSync('openssl', args, { cwd: directory, stdio: 'pipe' });
}
openssl('req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', 'ca.key',
  '-out', 'ca.crt', '-days', '1', '-subj', '/CN=Playwright smoke CA');
openssl('req', '-newkey', 'rsa:2048', '-nodes', '-keyout', 'server.key',
  '-out', 'server.csr', '-subj', '/CN=localhost');
fs.writeFileSync(`${directory}/server.ext`, 'subjectAltName=IP:127.0.0.1\nextendedKeyUsage=serverAuth\n');
openssl('x509', '-req', '-in', 'server.csr', '-CA', 'ca.crt', '-CAkey', 'ca.key',
  '-CAcreateserial', '-out', 'server.crt', '-days', '1', '-extfile', 'server.ext');
fs.writeFileSync(`${directory}/auth.json`, JSON.stringify({
  cookies: [{ name: 'smoke-session', value: 'synthetic-test-only', domain: '127.0.0.1',
    path: '/', expires: -1, httpOnly: false, secure: true, sameSite: 'Lax' }],
  origins: [],
}), { mode: 0o600 });
for (const name of fs.readdirSync(directory)) fs.chownSync(`${directory}/${name}`, 1000, 1000);
fs.chownSync(directory, 1000, 1000);
