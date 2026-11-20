// Shared subprocess execution defaults and binary builder for the documentation build scripts.

import { execFileSync } from 'node:child_process';

export const MAX_BUFFER = 64 * 1024 * 1024;

export const run = (command, args, options = {}) =>
  execFileSync(command, args, {
    encoding: 'utf8',
    maxBuffer: MAX_BUFFER,
    stdio: ['ignore', 'pipe', 'inherit'],
    ...options,
  });

/** Builds the xpd binary from `repository` once and returns its executable path. */
export function buildXpd(repository) {
  const messages = run('cargo', ['build', '--quiet', '--message-format=json-render-diagnostics'], {
    cwd: repository,
  });
  for (const line of messages.split('\n')) {
    if (!line.startsWith('{')) continue;
    const message = JSON.parse(line);
    if (message.reason === 'compiler-artifact' && message.target?.name === 'xpd' && message.executable) {
      return message.executable;
    }
  }
  throw new Error('cargo build named no xpd executable');
}
