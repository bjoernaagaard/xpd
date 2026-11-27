import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const BANNED_WORDS = [
  'leverage',
  'leveraging',
  'robust',
  'delve',
  'seamless',
  'seamlessly',
  'streamline',
  'streamlined',
  'pivotal',
  'landscape',
  'testament',
];

const BANNED_PHRASES = [
  "here's the thing",
  "it's worth noting",
  'it is worth noting',
  'as you can see',
  'the key point is',
  'not just ',
];

const FORBIDDEN_CHARS = [
  ['\u2014', 'em dash'],
  ['\u2013', 'en dash'],
  ['\u2018', 'curly single quote'],
  ['\u2019', 'curly apostrophe'],
  ['\u201c', 'curly double quote'],
  ['\u201d', 'curly double quote'],
  [';', 'semicolon'],
  ['\u00b7', 'middle dot'],
  ['\u2192', 'arrow'],
];

function collectFiles(repository) {
  const files = [
    join(repository, 'README.md'),
    join(repository, 'website/src/content/docs/index.md'),
    join(repository, 'rules/README.md'),
  ];
  const guidesDir = join(repository, 'website/src/content/docs/guides');
  if (existsSync(guidesDir)) {
    for (const entry of readdirSync(guidesDir)) {
      if (entry.endsWith('.md')) files.push(join(guidesDir, entry));
    }
  }
  const refDir = join(repository, 'website/reference');
  if (existsSync(refDir)) {
    for (const entry of readdirSync(refDir, { recursive: true })) {
      if (entry.endsWith('.md')) files.push(join(refDir, entry));
    }
  }
  const rulesDir = join(repository, 'rules');
  if (existsSync(rulesDir)) {
    for (const entry of readdirSync(rulesDir)) {
      const pack = join(rulesDir, entry, 'pack.md');
      if (existsSync(pack)) files.push(pack);
    }
  }
  return files.sort();
}

function stripInline(line) {
  return line
    .replace(/`[^`]*`/g, 'CODE')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/<https?:\/\/[^>]+>/g, 'URL')
    .replace(/https?:\/\/\S+/g, 'URL');
}

function checkFile(repository, file) {
  const rel = relative(repository, file);
  const raw = readFileSync(file, 'utf8');
  const lines = raw.split('\n');
  const issues = [];
  let index = 0;
  if (lines[0] === '---') {
    index = 1;
    while (index < lines.length && lines[index] !== '---') index += 1;
    index += 1;
  }
  let inFence = false;
  let fenceChar = '';
  let inHtmlComment = false;
  let firstProseLine = null;
  let prevEndedSentence = true;

  for (; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    const lineNo = index + 1;

    if (inHtmlComment) {
      if (trimmed.includes('-->')) inHtmlComment = false;
      continue;
    }
    if (trimmed.startsWith('<!--')) {
      if (!trimmed.includes('-->')) inHtmlComment = true;
      continue;
    }

    const fenceMatch = trimmed.match(/^(`{3,}|~{3,})/);
    if (fenceMatch) {
      if (!inFence) {
        inFence = true;
        fenceChar = fenceMatch[1][0];
      } else if (trimmed.startsWith(fenceChar.repeat(3))) {
        inFence = false;
        fenceChar = '';
      }
      prevEndedSentence = true;
      continue;
    }
    if (inFence) continue;
    if (!trimmed) {
      prevEndedSentence = true;
      continue;
    }
    if (trimmed.startsWith('#') || trimmed.startsWith('<') || /^- [a-z0-9-]+$/.test(trimmed)) {
      prevEndedSentence = true;
      continue;
    }

    const isTable = trimmed.startsWith('|');
    const clean = stripInline(line);
    if (firstProseLine === null && !isTable && !trimmed.startsWith('- ') && !/^\d+\.\s/.test(trimmed)) {
      firstProseLine = { lineNo, text: clean.trim() };
    }

    for (const [ch, label] of FORBIDDEN_CHARS) {
      if (clean.includes(ch)) {
        issues.push(`${rel}:${lineNo}: contains ${label}`);
      }
    }

    const lower = clean.toLowerCase();
    for (const word of BANNED_WORDS) {
      if (new RegExp(`\\b${word}\\b`).test(lower)) {
        issues.push(`${rel}:${lineNo}: banned machine-writing word "${word}"`);
      }
    }
    for (const phrase of BANNED_PHRASES) {
      if (lower.includes(phrase)) {
        issues.push(`${rel}:${lineNo}: banned machine-writing phrase "${phrase.trim()}"`);
      }
    }

    if (!isTable) {
      const strippedBullet = clean.replace(/^\s*(?:[-*]|\d+\.)\s+/, '');
      if (prevEndedSentence && /^(?:This|That|These|Those)\b/.test(strippedBullet)) {
        issues.push(`${rel}:${lineNo}: sentence starts with demonstrative pronoun`);
      }
      if (/[.!?]\s+(?:This|That|These|Those)\b/.test(clean)) {
        issues.push(`${rel}:${lineNo}: sentence starts with demonstrative pronoun`);
      }
      prevEndedSentence = /[.!?]["']?$/.test(trimmed);
    } else {
      prevEndedSentence = true;
    }
  }

  if (firstProseLine && /\?$/.test(firstProseLine.text.split(/[.!?]/)[0] ?? '')) {
    issues.push(`${rel}:${firstProseLine.lineNo}: opening line must be a BLUF statement, not a question`);
  }

  return issues;
}

/** Validates authored documentation files against the human-writing and BLUF rules. */
export function lintDocs(repository) {
  const files = collectFiles(repository);
  const issues = files.flatMap((file) => checkFile(repository, file));
  if (issues.length) {
    throw new Error(`prose-lint.mjs found ${issues.length} style issues:\n${issues.join('\n')}`);
  }
  console.log(`prose-lint.mjs: checked ${files.length} documentation files with 0 issues`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const repository = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
  lintDocs(repository);
}
