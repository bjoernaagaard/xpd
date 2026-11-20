// Shared subprocess execution defaults for the documentation build scripts.

import { execFileSync } from 'node:child_process';

export const MAX_BUFFER = 64 * 1024 * 1024;

export const run = (command, args, options = {}) =>
  execFileSync(command, args, {
    encoding: 'utf8',
    maxBuffer: MAX_BUFFER,
    stdio: ['ignore', 'pipe', 'inherit'],
    ...options,
  });
