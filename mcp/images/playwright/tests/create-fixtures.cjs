// Create disposable TLS/auth fixtures inside Docker, without host OpenSSL/Node.
const fs = require('node:fs');
const { execFileSync } = require('node:child_process');

const directory = process.env.FIXTURE_DIRECTORY || '/project/.ai-profiles/.private';
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
const authState = process.env.AUTH_STATE_FILE || '/project/auth.json';
fs.writeFileSync(authState, JSON.stringify({
  cookies: [{ name: 'smoke-session', value: 'synthetic-test-only', domain: '127.0.0.1',
    path: '/', expires: -1, httpOnly: false, secure: true, sameSite: 'Lax' }],
  origins: [],
}), { mode: 0o600 });
fs.chownSync(authState, 1000, 1000);
for (const name of fs.readdirSync(directory)) fs.chownSync(`${directory}/${name}`, 1000, 1000);
fs.chownSync(directory, 1000, 1000);
if (process.env.FIXTURE_DIRECTORY) {
  fs.renameSync(`${directory}/ca.crt`, `${directory}/rootCA.pem`);
  fs.renameSync(`${directory}/ca.key`, `${directory}/rootCA-key.pem`);
  fs.appendFileSync(`${directory}/rootCA-key.pem`, '\nT04_CA_PRIVATE_KEY_MUST_NOT_BE_READ\n');
  fs.chmodSync(`${directory}/rootCA-key.pem`, 0);
}
