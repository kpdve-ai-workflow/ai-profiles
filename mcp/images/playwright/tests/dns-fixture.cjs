// Synthetic, VM-only DNS: no forwarding and no dependency on corporate DNS.
const dgram = require('node:dgram');
const server = dgram.createSocket('udp4');
server.on('error', error => { console.error(error); process.exit(1); });
server.on('message', (message, remote) => {
  if (message.length < 12 || message.readUInt16BE(4) !== 1) return;
  let offset = 12;
  const labels = [];
  while (offset < message.length && message[offset] > 0 && message[offset] <= 63) {
    const length = message[offset++];
    if (offset + length > message.length) return;
    labels.push(message.subarray(offset, offset + length).toString('ascii'));
    offset += length;
  }
  if (message[offset] !== 0 || offset + 5 > message.length) return;
  offset++;
  const type = message.readUInt16BE(offset);
  const klass = message.readUInt16BE(offset + 2);
  const name = labels.join('.').toLowerCase();
  console.log(`QUERY ${name} ${type}`);
  const known = name === 'app.corp.test';
  const answer = known && type === 1 && klass === 1;
  const header = Buffer.from(message.subarray(0, 12));
  header.writeUInt16BE(known ? 0x8180 : 0x8183, 2);
  header.writeUInt16BE(answer ? 1 : 0, 6);
  header.writeUInt16BE(0, 8);
  header.writeUInt16BE(0, 10);
  const record = Buffer.from('c00c000100010000000000047f000001', 'hex');
  server.send(Buffer.concat([header, message.subarray(12, offset + 4), ...(answer ? [record] : [])]), remote.port, remote.address);
});
server.bind(53, '127.203.77.53', () => console.log('DNS_READY'));
