import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { buildXpd } from '../website/scripts/exec.mjs';

const binary = process.argv[2] ? resolve(process.argv[2]) : buildXpd(process.cwd());
const source = readFileSync(new URL('../samples/bad.py', import.meta.url), 'utf8');
const file = 'case.py';

const transitions = {
  empty: { pull: 'imported', checkout: 'imported', write: 'edited' },
  inherited: { touch: 'inherited', write: 'edited' },
  imported: { write: 'edited' },
  edited: { compact: 'compacted', commit: 'committed', resume: 'inherited' },
  compacted: { commit: 'committed' },
  committed: {},
};

function reachableTraces() {
  const queue = [
    ['empty', []],
    ['inherited', ['inherit']],
  ];
  const visited = new Set();
  const traces = [];
  for (const [state, path] of queue) {
    traces.push([state, path]);
    if (visited.has(state)) continue;
    visited.add(state);
    for (const [action, next] of Object.entries(transitions[state])) {
      queue.push([next, [...path, action]]);
    }
  }
  return traces;
}

function replay(root, actions) {
  const upstream = join(root, 'upstream');
  const cwd = join(root, 'checkout');
  const env = {
    ...process.env,
    XDG_CACHE_HOME: join(root, 'cache'),
    GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_CONFIG_NOSYSTEM: '1',
    GIT_AUTHOR_NAME: 'Fixture',
    GIT_COMMITTER_NAME: 'Fixture',
    GIT_AUTHOR_EMAIL: 'fixture@example.org',
    GIT_COMMITTER_EMAIL: 'fixture@example.org',
  };
  const git = (dir, ...args) => execFileSync('git', args, { cwd: dir, env, stdio: 'pipe' });
  const hook = (event, source) =>
    execFileSync(binary, ['hook'], {
      cwd,
      env,
      encoding: 'utf8',
      input: JSON.stringify({ hook_event_name: event, cwd, session_id: 'test', source }),
    }).trim();
  mkdirSync(upstream);
  git(upstream, 'init', '-b', 'main');
  git(upstream, 'commit', '--allow-empty', '-m', 'initial');
  git(root, 'clone', upstream, cwd);
  writeFileSync(join(upstream, file), source);
  git(upstream, 'add', file);
  git(upstream, 'commit', '-m', 'foreign file');
  const path = join(cwd, file);
  if (actions.includes('checkout')) {
    git(cwd, 'switch', '-c', 'earlier');
    writeFileSync(path, source);
    git(cwd, 'add', file);
    git(cwd, 'commit', '-m', 'work before session start');
    git(cwd, 'switch', 'main');
  }
  if (actions[0] === 'inherit') {
    writeFileSync(path, source);
    utimesSync(path, 1, 1);
  }
  hook('SessionStart', 'startup');
  for (const action of actions) {
    switch (action) {
      case 'inherit':
        break;
      case 'touch':
        utimesSync(path, new Date(), new Date());
        break;
      case 'checkout':
        git(cwd, 'switch', 'earlier');
        break;
      case 'pull':
        git(cwd, 'pull', '--ff-only');
        break;
      case 'write':
        writeFileSync(path, source + '\nanswer = 42\n');
        utimesSync(path, 1, 1);
        break;
      case 'commit':
        git(cwd, 'add', file);
        git(cwd, 'commit', '-m', 'local edit');
        break;
      case 'compact':
      case 'resume':
        hook('SessionStart', action);
        break;
      default:
        throw Error(action);
    }
  }
  const reply = hook('Stop');
  return reply ? JSON.parse(reply).decision === 'block' : false;
}

let failures = 0;
for (const [state, actions] of reachableTraces()) {
  const root = mkdtempSync(join(tmpdir(), 'xpd-stop-'));
  try {
    const expected = ['edited', 'compacted', 'committed'].includes(state);
    const observed = replay(root, actions);
    const trace = [
      actions[0] === 'inherit' ? 'inherit, start' : 'start',
      ...actions.filter((action) => action !== 'inherit'),
      'stop',
    ].join(', ');
    if (observed !== expected) {
      console.error(
        `${trace}: expected ${expected ? 'block' : 'allow'}, got ${observed ? 'block' : 'allow'}`,
      );
      failures++;
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}
if (failures) process.exitCode = 1;
else console.log(`Stop matches all ${reachableTraces().length} modeled traces`);
