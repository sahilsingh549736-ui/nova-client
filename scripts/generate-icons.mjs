import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const size = 256;
const raw = Buffer.alloc((size * 4 + 1) * size);
for (let y = 0; y < size; y++) {
  const row = y * (size * 4 + 1);
  raw[row] = 0;
  for (let x = 0; x < size; x++) {
    const dx = x - 127.5, dy = y - 127.5;
    const r = Math.sqrt(dx * dx + dy * dy);
    const i = row + 1 + x * 4;
    const glow = Math.max(0, 1 - r / 190);
    raw[i] = Math.round(10 + glow * 24);
    raw[i + 1] = Math.round(14 + glow * 38);
    raw[i + 2] = Math.round(30 + glow * 100);
    raw[i + 3] = 255;
    // A simple cyan four-point star mark.
    const ax = Math.abs(dx), ay = Math.abs(dy);
    const star = (ax < 13 && ay < 66 && ay > 15) ||
      (ay < 13 && ax < 66 && ax > 15) ||
      (ax < 31 && ay < 31 && ax + ay < 48);
    if (star) { raw[i] = 115; raw[i + 1] = 220; raw[i + 2] = 255; }
  }
}
const crcTable = new Uint32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
  crcTable[n] = c >>> 0;
}
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const name = Buffer.from(type);
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  name.copy(out, 4); data.copy(out, 8);
  out.writeUInt32BE(crc32(Buffer.concat([name, data])), 8 + data.length);
  return out;
};
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4);
ihdr[8] = 8; ihdr[9] = 6;
const png = Buffer.concat([
  Buffer.from([137,80,78,71,13,10,26,10]),
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(raw)),
  chunk("IEND", Buffer.alloc(0))
]);
const icoHeader = Buffer.alloc(22);
icoHeader.writeUInt16LE(0, 0); icoHeader.writeUInt16LE(1, 2); icoHeader.writeUInt16LE(1, 4);
icoHeader[6] = 0; icoHeader[7] = 0; icoHeader[8] = 0; icoHeader[9] = 0;
icoHeader.writeUInt16LE(1, 10); icoHeader.writeUInt16LE(32, 12);
icoHeader.writeUInt32LE(png.length, 14); icoHeader.writeUInt32LE(22, 18);
const dir = path.join("src-tauri", "icons");
fs.mkdirSync(dir, { recursive: true });
fs.writeFileSync(path.join(dir, "icon.png"), png);
fs.writeFileSync(path.join(dir, "icon.ico"), Buffer.concat([icoHeader, png]));
console.log("Generated Nova Client PNG and ICO bundle icons.");
