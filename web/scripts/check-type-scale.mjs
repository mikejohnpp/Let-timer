import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const srcDir = process.argv[2] ? path.resolve(process.argv[2]) : path.join(projectRoot, 'src');

// Forbid sub-12px font sizes like text-[10px], text-[11px], text-[9px] etc.
const SUB_12PX_REGEX = /text-\[\s*([0-9]+(?:\.[0-9]+)?)\s*px\s*\]/g;
const ALLOW_REGEX = /(?:\/\/|\/\*)\s*type-allow:\s*(\S+.*)/;

function isExcluded(relPath) {
  const normalized = relPath.replace(/\\/g, '/');
  if (normalized.startsWith('test/') || normalized.includes('/test/')) return true;
  return false;
}

let totalViolations = 0;

function scanDir(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      scanDir(fullPath);
    } else if (entry.isFile()) {
      const ext = path.extname(entry.name);
      if (!['.ts', '.tsx', '.css'].includes(ext)) continue;

      const relPath = path.relative(srcDir, fullPath);
      if (isExcluded(relPath)) continue;

      const content = fs.readFileSync(fullPath, 'utf8');
      const lines = content.split('\n');

      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];

        const hasCurrentLineAllow = ALLOW_REGEX.test(line);
        const hasPrevLineAllow = i > 0 && ALLOW_REGEX.test(lines[i - 1]);
        if (hasCurrentLineAllow || hasPrevLineAllow) continue;

        let match;
        // Reset regex index
        SUB_12PX_REGEX.lastIndex = 0;
        while ((match = SUB_12PX_REGEX.exec(line)) !== null) {
          const size = parseFloat(match[1]);
          if (size < 12) {
            console.error(
              `\x1b[31m[TYPE SCALE LINT ERROR]\x1b[0m ${relPath}:${i + 1} - Found sub-12px size: "${match[0]}" in "${line.trim()}"`
            );
            totalViolations++;
          }
        }
      }
    }
  }
}

console.log('Scanning web/src for sub-12px text sizes...');
scanDir(srcDir);

if (totalViolations > 0) {
  console.error(`\n\x1b[31mFAIL: Found ${totalViolations} type scale violations.\x1b[0m`);
  console.error('Approved scale: 12 / 14 / 16 / 20 / 28 / 48 / 72 px (= xs / sm / base / lg+ / display).');
  console.error('If intentionally allowed (e.g. dense metadata), annotate with: `// type-allow: <reason>`\n');
  process.exit(1);
} else {
  console.log('\x1b[32mPASS: All typography sizes comply with type scale (>= 12px or annotated).\x1b[0m\n');
  process.exit(0);
}
