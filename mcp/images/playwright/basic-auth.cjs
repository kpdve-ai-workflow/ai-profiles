'use strict';

// Never print parser errors: their messages can contain private file contents.
const fs = require('node:fs');
try {
  const credentials = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  if (!credentials || Array.isArray(credentials) ||
      Object.keys(credentials).sort().join(',') !== 'origin,password,username' ||
      typeof credentials.username !== 'string' || !credentials.username ||
      typeof credentials.password !== 'string' || !credentials.password ||
      typeof credentials.origin !== 'string' ||
      !/^https?:\/\/[^\s\\/?#]+$/.test(credentials.origin)) throw new Error();
  const origin = new URL(credentials.origin);
  if (!origin.hostname || origin.username || origin.password) throw new Error();
  // Normalize default ports to the browser's origin representation.
  credentials.origin = origin.origin;
  fs.writeFileSync(process.argv[3], JSON.stringify({
    browser: { contextOptions: { httpCredentials: credentials } },
  }), { mode: 0o600, flag: 'wx' });
} catch {
  fs.writeSync(2, 'playwright: Basic Auth requires a readable private JSON file with non-empty username, password and an HTTP(S) origin (scheme://host:port); configuration was not loaded\n');
  process.exitCode = 2;
}
