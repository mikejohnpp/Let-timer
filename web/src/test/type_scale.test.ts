import { describe, it, expect } from 'vitest';
import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';
import os from 'os';

describe('T1: check-type-scale script', () => {
  const scriptPath = path.resolve(__dirname, '../../scripts/check-type-scale.mjs');

  it('exits non-zero on a fixture containing forbidden text-[10px]', () => {
    const tempFixtureDir = path.join(os.tmpdir(), `check-type-scale-test-${Date.now()}-1`);
    fs.mkdirSync(tempFixtureDir, { recursive: true });
    const tempFixtureFile = path.join(tempFixtureDir, 'BadComponent.tsx');
    fs.writeFileSync(tempFixtureFile, 'export const Bad = () => <div className="text-[10px]">Small</div>;\n');

    try {
      let threw = false;
      try {
        execSync(`node "${scriptPath}" "${tempFixtureDir}"`, {
          cwd: path.resolve(__dirname, '../..'),
          stdio: 'pipe',
        });
      } catch (e: any) {
        threw = true;
        expect(e.status).not.toBe(0);
      }
      expect(threw).toBe(true);
    } finally {
      fs.rmSync(tempFixtureDir, { recursive: true, force: true });
    }
  });

  it('exits 0 when sub-12px size is annotated with // type-allow', () => {
    const tempFixtureDir = path.join(os.tmpdir(), `check-type-scale-test-${Date.now()}-2`);
    fs.mkdirSync(tempFixtureDir, { recursive: true });
    const tempFixtureFile = path.join(tempFixtureDir, 'AllowedComponent.tsx');
    fs.writeFileSync(
      tempFixtureFile,
      '// type-allow: dense timestamp badge\nexport const Allowed = () => <div className="text-[10px]">Small</div>;\n'
    );

    try {
      const output = execSync(`node "${scriptPath}" "${tempFixtureDir}"`, {
        cwd: path.resolve(__dirname, '../..'),
        stdio: 'pipe',
      }).toString();
      expect(output).toContain('PASS');
    } finally {
      fs.rmSync(tempFixtureDir, { recursive: true, force: true });
    }
  });

  it('exits 0 on standard scale sizes (text-xs, text-sm)', () => {
    const tempFixtureDir = path.join(os.tmpdir(), `check-type-scale-test-${Date.now()}-3`);
    fs.mkdirSync(tempFixtureDir, { recursive: true });
    const tempFixtureFile = path.join(tempFixtureDir, 'GoodComponent.tsx');
    fs.writeFileSync(tempFixtureFile, 'export const Good = () => <div className="text-xs font-medium">Standard</div>;\n');

    try {
      const output = execSync(`node "${scriptPath}" "${tempFixtureDir}"`, {
        cwd: path.resolve(__dirname, '../..'),
        stdio: 'pipe',
      }).toString();
      expect(output).toContain('PASS');
    } finally {
      fs.rmSync(tempFixtureDir, { recursive: true, force: true });
    }
  });
});

