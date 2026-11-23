import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { relative } from 'node:path';

import { parseMarkdown } from '../src/markdown.mjs';

export function text(value) {
  return String(value ?? '')
    .split(/(`[^`]*`)/)
    .map((part, index) =>
      index % 2 ? part : part.replace(/[\\*_[\]|]/g, '\\$&').replace(/</g, '&lt;').replace(/>/g, '&gt;'),
    )
    .join('');
}

/** Help text with its line breaks joined, keeping blank lines and list items. */
export function reflow(value) {
  return String(value ?? '')
    .trim()
    .split(/\n\s*\n/)
    .map((paragraph) =>
      paragraph
        .split('\n')
        .map((line) => line.trim())
        .reduce((lines, line) => {
          if (lines.length && !/^[-*] /.test(line)) lines[lines.length - 1] += ` ${line}`;
          else lines.push(line);
          return lines;
        }, [])
        .map(text)
        .join('\n'),
    )
    .join('\n\n');
}

export const cell = (value) => reflow(value).replaceAll('\n', ' ');
export const code = (value) => `\`${String(value).replaceAll('`', '')}\``;

/** An authored file split into the text before its first `## ` heading and its sections. */
export function readAuthored(path) {
  const source = readFileSync(path, 'utf8');
  const { intro, sections } = parseMarkdown(source);
  return { source, intro, sections };
}

function markdownFiles(folder) {
  if (!existsSync(folder)) return [];
  return readdirSync(folder, { recursive: true })
    .filter((file) => file.endsWith('.md'))
    .map((file) => file.replaceAll('\\', '/').replace(/\.md$/, ''));
}

/** Fails when the names of the authored files and the names of the listed items differ. */
export function validateNames(repository, what, folder, listed) {
  const files = markdownFiles(folder);
  const missing = listed.filter((name) => !files.includes(name));
  const orphans = files.filter((name) => !listed.includes(name));
  if (missing.length || orphans.length) {
    throw new Error(
      `${relative(repository, folder)} needs one Markdown file for each ${what}. ` +
        `Missing: ${missing.join(', ') || 'none'}. Naming no ${what}: ${orphans.join(', ') || 'none'}.`,
    );
  }
}
