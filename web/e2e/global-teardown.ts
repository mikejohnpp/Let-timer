import fs from 'fs';
import { E2E_STATE_FILE } from './global-setup';

export default async function globalTeardown() {
  if (fs.existsSync(E2E_STATE_FILE)) {
    try {
      const state = JSON.parse(fs.readFileSync(E2E_STATE_FILE, 'utf-8'));
      if (state.daemonPid) {
        try {
          process.kill(state.daemonPid, 'SIGTERM');
        } catch {
          // ignore if already exited
        }
      }
      if (state.tempDir && fs.existsSync(state.tempDir)) {
        fs.rmSync(state.tempDir, { recursive: true, force: true });
      }
      fs.unlinkSync(E2E_STATE_FILE);
    } catch (e) {
      console.warn('Error during e2e global teardown:', e);
    }
  }
}

