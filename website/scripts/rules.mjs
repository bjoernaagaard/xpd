// Writes one page for each rule, one page for each pack and the rules overview into
// src/content/docs/rules/, from the catalog of the xpd that this checkout builds.
//
// The rule text lives in rules/<pack>/<id>.md, which Cargo embeds in xpd. This script reads it
// back from `xpd rules --builtin --format json`, so the site shows what an installed xpd prints.
// The pack pages come from rules/<pack>/pack.md, and the overview from rules/README.md.

import { execFileSync } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PACKS } from '../src/packs.mjs';

const website = join(dirname(fileURLToPath(import.meta.url)), '..');
const repository = join(website, '..');
const rulesFolder = join(repository, 'rules');
const out = join(website, 'src', 'content', 'docs', 'rules');
const editBase = 'https://github.com/bjoernaagaard/xpd/edit/main/rules';

const catalog = JSON.parse(
  execFileSync('cargo', ['run', '--quiet', '--', 'rules', '--builtin', '--format', 'json'], {
    cwd: repository,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'inherit'],
  }),
);

const folders = readdirSync(rulesFolder, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);
const unlisted = folders.filter((folder) => !PACKS.includes(folder));
const missing = PACKS.filter((pack) => !folders.includes(pack));
if (unlisted.length || missing.length) {
  throw new Error(
    `src/packs.mjs must list each folder under rules/ once. Missing: ${unlisted.join(', ') || 'none'}. Without a folder: ${missing.join(', ') || 'none'}.`,
  );
}

const byPack = new Map(PACKS.map((pack) => [pack, []]));
for (const rule of catalog) {
  if (!byPack.has(rule.package)) {
    throw new Error(`xpd rules lists ${rule.package}/${rule.rule}, whose pack has no folder`);
  }
  if (!rule.doc) {
    throw new Error(`xpd rules gives ${rule.package}/${rule.rule} no doc`);
  }
  byPack.get(rule.package).push(rule);
}

/** The text of a YAML scalar, as JSON, which YAML reads as the same string. */
const yaml = (text) => JSON.stringify(text);

/** The first sentence of the first paragraph of a rule's Markdown, as plain text. */
function summary(doc) {
  const paragraph = doc.split('\n\n')[0].replaceAll('\n', ' ');
  const sentence = paragraph.match(/^.*?[.!?](?=\s|$)/)?.[0] ?? paragraph;
  return sentence.replaceAll('`', '');
}

/**
 * Turns the links between Markdown files under rules/ into links between site pages. `depth` is
 * the number of folders between the page and the rules overview: 1 for a pack page, 2 for a rule
 * page, whose address ends in /rules/<pack>/<id>/.
 */
function siteLinks(markdown, depth) {
  const up = '../'.repeat(depth);
  const fromPack = depth === 1 ? './' : '../';
  return markdown.replace(/\]\(((?:\.\.\/([a-z_]+)\/)?([a-z0-9-]+|pack)\.md)(#[^)]*)?\)/g, (all, _path, pack, name, fragment = '') => {
    const target = pack ? `${up}${pack}/` : fromPack;
    return `](${name === 'pack' ? target : `${target}${name}/`}${fragment})`;
  });
}

function fixText(rule) {
  switch (rule.fix) {
    case 'safe':
      return 'safe, with `xpd check --fix`';
    case 'unsafe':
      return 'unsafe, with `xpd check --fix --unsafe-fixes`';
    default:
      return 'none, edit by hand';
  }
}

function page(frontmatter, body) {
  const lines = Object.entries(frontmatter).map(([key, value]) => `${key}: ${value}`);
  return `---\n${lines.join('\n')}\n---\n\n${body.trim()}\n`;
}

/** The pack page's title and its Markdown after the title. */
function packPage(pack) {
  const text = readFileSync(join(rulesFolder, pack, 'pack.md'), 'utf8');
  const [first, ...rest] = text.split('\n');
  if (!first.startsWith('# ')) {
    throw new Error(`rules/${pack}/pack.md must start with "# <pack name>"`);
  }
  return { title: first.slice(2).trim(), body: rest.join('\n') };
}

rmSync(out, { recursive: true, force: true });
const titles = new Map(PACKS.map((pack) => [pack, packPage(pack).title]));

for (const [pack, rules] of byPack) {
  const folder = join(out, pack);
  mkdirSync(folder, { recursive: true });
  const title = titles.get(pack);
  for (const rule of rules) {
    const versions = rule.since ? `${title} ${rule.since} and later` : 'every version';
    const body = [
      `| Rule | ${rule.rule} |`,
      '| --- | --- |',
      `| Pack | [${title}](../) |`,
      `| Severity | ${rule.severity} |`,
      `| Fix | ${fixText(rule)} |`,
      `| Library versions | ${versions} |`,
      '',
      siteLinks(rule.doc, 2),
      '',
      '## Message',
      '',
      'xpd prints this message with the issue. A name that starts with `$` stands for code from your file.',
      '',
      '```text',
      rule.message,
      '```',
      '',
      `Browse the other rules of the [${title} pack](../).`,
    ].join('\n');
    writeFileSync(
      join(folder, `${rule.rule}.md`),
      page(
        {
          title: yaml(`${pack}/${rule.rule}`),
          description: yaml(summary(rule.doc)),
          editUrl: yaml(`${editBase}/${pack}/${rule.rule}.md`),
        },
        body,
      ),
    );
  }

  const ids = new Map(rules.map((rule) => [rule.rule, rule]));
  const listed = new Set();
  const lines = [];
  let table = [];
  const flush = () => {
    if (table.length) {
      lines.push('| Rule | Severity | Fix | What it finds |', '| --- | --- | --- | --- |', ...table);
      table = [];
    }
  };
  let inCode = false;
  for (const line of packPage(pack).body.split('\n')) {
    if (line.trimStart().startsWith('```')) inCode = !inCode;
    const id = !inCode && line.match(/^- ([a-z0-9-]+)$/)?.[1];
    if (id && ids.has(id)) {
      const rule = ids.get(id);
      listed.add(id);
      table.push(`| [${id}](./${id}/) | ${rule.severity} | ${rule.fix ?? 'none'} | ${summary(rule.doc).replaceAll('|', '\\|')} |`);
      continue;
    }
    flush();
    lines.push(line);
  }
  flush();
  const unlistedRules = rules.filter((rule) => !listed.has(rule.rule)).map((rule) => rule.rule);
  if (unlistedRules.length) {
    throw new Error(`rules/${pack}/pack.md does not list ${unlistedRules.join(', ')}`);
  }
  const fixes = rules.filter((rule) => rule.fix).length;
  const intro = `The ${title} pack has ${rules.length} ${rules.length === 1 ? 'rule' : 'rules'}, and xpd can fix ${fixes} of them.`;
  writeFileSync(
    join(folder, 'index.md'),
    page(
      {
        title: yaml(title),
        description: yaml(`The ${rules.length} xpd ${rules.length === 1 ? 'rule' : 'rules'} for ${title}.`),
        editUrl: yaml(`${editBase}/${pack}/pack.md`),
      },
      `${intro}\n\n${siteLinks(lines.join('\n'), 1)}`,
    ),
  );
}

const overview = readFileSync(join(rulesFolder, 'README.md'), 'utf8').replace(/^# .*\n/, '');
const table = [
  '| Pack | Rules | Safe fix | Unsafe fix | No fix |',
  '| --- | ---: | ---: | ---: | ---: |',
  ...PACKS.map((pack) => {
    const rules = byPack.get(pack);
    const count = (fix) => rules.filter((rule) => rule.fix === fix).length;
    return `| [${titles.get(pack)}](./${pack}/) | ${rules.length} | ${count('safe')} | ${count('unsafe')} | ${count(null)} |`;
  }),
  `| All packs | ${catalog.length} | ${catalog.filter((r) => r.fix === 'safe').length} | ${catalog.filter((r) => r.fix === 'unsafe').length} | ${catalog.filter((r) => !r.fix).length} |`,
];
writeFileSync(
  join(out, 'index.md'),
  page(
    {
      title: yaml('Rules'),
      description: yaml(`The ${catalog.length} rules that xpd checks, by pack.`),
      editUrl: yaml(`${editBase}/README.md`),
    },
    `${overview.trim()}\n\n${table.join('\n')}`,
  ),
);

console.log(`rules.mjs: wrote ${catalog.length} rule pages and ${PACKS.length} pack pages`);
