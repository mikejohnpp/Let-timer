import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const srcDir = process.argv[2] ? path.resolve(process.argv[2]) : path.join(projectRoot, 'src');

const FORBIDDEN_PATTERNS = [
  { name: 'raw duration', regex: /duration:\s*0?\.\d/ },
  { name: 'raw ease array', regex: /ease:\s*\[/ },
  { name: 'cubic-bezier', regex: /cubic-bezier\(/ },
  { name: 'infinite/banned Tailwind animation', regex: /animate-(ping|pulse|bounce)/ },
  { name: 'raw transition-duration', regex: /transition-duration:\s*\d/ },
];

const ALLOW_REGEX = /(?:\/\/|\/\*)\s*motion-allow:\s*(\S+.*)/;

function isExcluded(relPath) {
  const normalized = relPath.replace(/\\/g, '/');
  if (normalized.startsWith('test/') || normalized.includes('/test/')) return true;
  if (normalized === 'motion/tokens.ts') return true;
  if (normalized === 'styles/motion.css') return true;
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

        // Check for permission comment on the current line or previous line
        const hasCurrentLineAllow = ALLOW_REGEX.test(line);
        const hasPrevLineAllow = i > 0 && ALLOW_REGEX.test(lines[i - 1]);
        if (hasCurrentLineAllow || hasPrevLineAllow) continue;

        for (const { name, regex } of FORBIDDEN_PATTERNS) {
          if (regex.test(line)) {
            console.error(`\x1b[31m[MOTION LINT ERROR]\x1b[0m ${relPath}:${i + 1} - Found ${name}: "${line.trim()}"`);
            totalViolations++;
          }
        }
      }
    }
  }
}

console.log('Scanning web/src for forbidden motion tokens and raw timings...');
scanDir(srcDir);

if (totalViolations > 0) {
  console.error(`\n\x1b[31mFAIL: Found ${totalViolations} motion token violations.\x1b[0m`);
  console.error('Use tokens from `useMotionTransition`, `tokens.ts`, or `motion.css`.');
  console.error('If intentionally allowed, annotate with: `// motion-allow: <reason>`\n');
  process.exit(1);
} else {
  console.log('\x1b[32mPASS: All motion tokens and timings comply with motion_plan.md.\x1b[0m\n');
  process.exit(0);
}
