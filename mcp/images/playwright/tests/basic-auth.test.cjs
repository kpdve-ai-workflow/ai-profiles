const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const helper = process.env.BASIC_AUTH_HELPER || path.join(__dirname, '../basic-auth.cjs');
const credentials = { username: 'fixture-user', password: 'fixture-secret', origin: 'https://example.test:443' };

function invoke(t, input) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'basic-auth-test-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const source = path.join(directory, 'credentials.json');
  const output = path.join(directory, 'config.json');
  if (input !== undefined) fs.writeFileSync(source, input, { mode: 0o600 });
  const result = spawnSync(process.execPath, [helper, source, output], { encoding: 'utf8' });
  return { ...result, output, source };
}

test('creates a private MCP configuration with credentials restricted to normalized origin', t => {
  const result = invoke(t, JSON.stringify(credentials));
  assert.equal(result.status, 0);
  assert.equal(result.stdout, '');
  assert.equal(result.stderr, '');
  assert.equal(fs.statSync(result.output).mode & 0o777, 0o600);
  assert.deepEqual(JSON.parse(fs.readFileSync(result.output)), {
    browser: { contextOptions: { httpCredentials: { ...credentials, origin: 'https://example.test' } } },
  });
  assert.deepEqual(JSON.parse(fs.readFileSync(result.source)), credentials);
});

test('rejects missing, malformed and unrestricted credentials without disclosing their contents', t => {
  const invalid = [
    undefined, '{"password":"fixture-secret",', 'null', '[]',
    ...[undefined, '', 'https://example.test/path', 'https://example.test/',
      'https://example.test?query', 'https://example.test#fragment',
      'https://fixture-user:fixture-secret@example.test', 'file://example.test',
      'https://example.test\\path', 'https://example.test\n'].map(origin => JSON.stringify({ ...credentials, origin })),
    JSON.stringify({ ...credentials, username: '' }),
    JSON.stringify({ ...credentials, password: 123 }),
    JSON.stringify({ ...credentials, extra: 'fixture-secret' }),
  ];
  for (const input of invalid) {
    const result = invoke(t, input);
    assert.equal(result.status, 2);
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /^playwright: Basic Auth requires/);
    assert.ok(!result.stderr.includes('fixture-secret'));
    assert.ok(!result.stderr.includes('fixture-user'));
    assert.ok(!fs.existsSync(result.output));
  }
});

test('preserves an explicit non-default port', t => {
  const result = invoke(t, JSON.stringify({ ...credentials, origin: 'http://example.test:8080' }));
  assert.equal(result.status, 0);
  assert.equal(JSON.parse(fs.readFileSync(result.output)).browser.contextOptions.httpCredentials.origin,
    'http://example.test:8080');
});
