// Local-only QR generation. The resulting SVG contains the private device key.
import fs from 'node:fs/promises';
import QRCode from '../apps/rain-client/node_modules/qrcode/lib/index.js';
const [source,destination]=process.argv.slice(2);
if(!source||!destination)throw Error('Usage: node scripts/rain-profile-qr.mjs profile.conf profile.svg');
const profile=await fs.readFile(source,'utf8');
await fs.writeFile(destination,await QRCode.toString(profile,{type:'svg',errorCorrectionLevel:'M',margin:2}),{mode:0o600,flag:'wx'});
