import { spawn } from 'child_process';
import fs from 'fs';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const E2E_STATE_FILE = path.join(os.tmpdir(), 'let-timer-e2e-state.json');

export default async function globalSetup() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'lt-e2e-'));
  const dbPath = path.join(tempDir, 'e2e.db');
  const sockPath = path.join(tempDir, 'e2e.sock');
  const daemonPort = 8799;
  const token = 'e2e-test-token';

  process.env.VITE_API_TARGET = `http://127.0.0.1:${daemonPort}`;
  process.env.VITE_PORT = '5174';

  const projectRoot = path.resolve(__dirname, '../..');
  const daemonBin = path.join(projectRoot, 'target/debug/let-timer-daemon');

  // Launch test daemon with isolated throw-away DB and socket
  const daemon = spawn(daemonBin, ['--http', `127.0.0.1:${daemonPort}`], {
    env: {
      ...process.env,
      LET_TIMER_DB: dbPath,
      LET_TIMER_SOCKET: sockPath,
      LET_TIMER_BOOTSTRAP_TOKEN: token,
      RUST_LOG: 'warn',
    },
    stdio: 'ignore',
  });

  fs.writeFileSync(
    E2E_STATE_FILE,
    JSON.stringify({
      tempDir,
      daemonPid: daemon.pid,
      daemonPort,
      token,
    })
  );

  // Poll daemon health endpoint until it is ready
  let healthy = false;
  for (let i = 0; i < 40; i++) {
    try {
      const res = await fetch(`http://127.0.0.1:${daemonPort}/api/v1/health`);
      if (res.ok) {
        healthy = true;
        break;
      }
    } catch {
      // waiting for daemon startup
    }
    await new Promise((r) => setTimeout(r, 250));
  }

  if (!healthy) {
    if (daemon.pid) daemon.kill('SIGKILL');
    throw new Error(`Test let-timer-daemon failed to become healthy on port ${daemonPort}`);
  }

  // Seed test data via REST API
  const apiBase = `http://127.0.0.1:${daemonPort}/api/v1`;
  const headers = {
    'Content-Type': 'application/json',
    'X-Let-Timer-Protocol': '1',
    Authorization: `Bearer ${token}`,
  };

  // 1. Create Media items (image, music, poem)
  await fetch(`${apiBase}/medias`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Forest Mist',
      url: 'https://images.unsplash.com/photo-1448375240586-882707db888b?auto=format&fit=crop&w=1200&q=80',
      media_type: 'image',
      description: 'Forest background',
    }),
  });

  const musicRes = await fetch(`${apiBase}/medias`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Gentle Rain',
      url: 'https://cdn.freesound.org/previews/531/531947_11861866-lq.mp3',
      media_type: 'music',
      description: 'Rain audio track',
    }),
  });
  const musicData = (await musicRes.json().catch(() => ({}))) as { id?: number };

  await fetch(`${apiBase}/medias`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'The Road Not Taken',
      url: 'urn:poem:frost',
      media_type: 'poem',
      description: 'Robert Frost poem',
      content: 'Two roads diverged in a yellow wood,\nAnd sorry I could not travel both',
    }),
  });

  // 2. Create Media List
  const listRes = await fetch(`${apiBase}/media-lists`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Deep Study',
      description: 'Study music and wallpapers',
    }),
  });
  const listData = (await listRes.json().catch(() => ({}))) as { id?: number };

  if (listData?.id && musicData?.id) {
    await fetch(`${apiBase}/media-lists/${listData.id}/medias`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ media_id: musicData.id }),
    });
  }

  // 3. Create Tasks (pending, running, completed, no-estimate, overtime)
  const d = new Date();
  const today = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

  // Focus running task
  const t1Res = await fetch(`${apiBase}/tasks`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Focus deep work',
      estimated_mins: 25,
      scheduled_on: today,
      priority: 2,
      media_list_id: listData?.id || null,
    }),
  });
  const t1 = (await t1Res.json().catch(() => ({}))) as { id?: number };
  if (t1?.id) {
    await fetch(`${apiBase}/tasks/${t1.id}/start`, { method: 'POST', headers });
  }

  // Pending task
  await fetch(`${apiBase}/tasks`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Review pull request',
      estimated_mins: 15,
      scheduled_on: today,
      priority: 1,
    }),
  });

  // Completed task
  const t3Res = await fetch(`${apiBase}/tasks`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Finished research',
      estimated_mins: 20,
      scheduled_on: today,
      priority: 0,
    }),
  });
  const t3 = (await t3Res.json().catch(() => ({}))) as { id?: number };
  if (t3?.id) {
    await fetch(`${apiBase}/tasks/${t3.id}/start`, { method: 'POST', headers });
    await fetch(`${apiBase}/tasks/${t3.id}/done`, { method: 'POST', headers });
  }

  // No-estimate task
  await fetch(`${apiBase}/tasks`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: 'Brainstorm ideas',
      estimated_mins: null,
      scheduled_on: today,
      priority: 0,
    }),
  });
}

