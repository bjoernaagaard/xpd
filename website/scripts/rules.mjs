// Writes one page for each rule, one page for each pack and the rules overview into
// src/content/docs/rules/, from the catalog of the xpd that this checkout builds.
//
// The rule text lives in rules/<pack>/<id>.md, which Cargo embeds in xpd. This script reads it
// back from `xpd rules --builtin --format json`, so the site shows what an installed xpd prints.
// The pack pages come from rules/<pack>/pack.md, and the overview from rules/README.md.

import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { editUrl, firstSentence, page, parseMarkdown, splitTitle, yaml } from '../src/markdown.mjs';
import { orderedPacks, packPage } from '../src/packs.mjs';
import { buildXpd, run } from './exec.mjs';

const website = join(dirname(fileURLToPath(import.meta.url)), '..');
const repository = join(website, '..');
const rulesFolder = join(repository, 'rules');
const out = join(website, 'src', 'content', 'docs', 'rules');

/** The first sentence of the first paragraph of a rule's Markdown, as plain text. */
const summary = (doc) => firstSentence(doc).replaceAll('`', '');

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

/** The severity of a rule, and for a rule of the house style the one under the strict setting. */
function severityText(rule) {
  if (rule.style !== 'house') return rule.severity;
  const strict = rule.severities.strict === rule.severities.default ? '' : `, and ${rule.severities.strict} with \`style = "strict"\``;
  return `${rule.severities.default}${strict}. A rule of the [house style](../../../commands/check/#house-style)`;
}

export function generateRules(xpd) {
  const catalog = JSON.parse(
    run(xpd, ['rules', '--builtin', '--format', 'json'], { cwd: repository }),
  );

  const grouped = new Map();
  for (const rule of catalog) {
    if (!rule.doc) {
      throw new Error(`xpd rules gives ${rule.package}/${rule.rule} no doc`);
    }
    const list = grouped.get(rule.package) ?? [];
    list.push(rule);
    grouped.set(rule.package, list);
  }

  const packs = orderedPacks(grouped.keys());
  const byPack = new Map(packs.map((pack) => [pack, grouped.get(pack)]));
  const ruleIdsByPack = new Map(packs.map((pack) => [pack, new Set(byPack.get(pack).map((r) => r.rule))]));

  /**
   * Turns relative links between rule/pack Markdown files into links between site pages,
   * matching only packs and rule ids reported by `xpd rules`.
   */
  function siteLinks(markdown, currentPack, depth) {
    const up = '../'.repeat(depth);
    const fromPack = depth === 1 ? './' : '../';
    return markdown.replace(
      /\]\(((?:\.\.\/([^/)]+)\/)?([^/)]+)\.md)(#[^)]*)?\)/g,
      (all, _path, linkedPack, name, fragment = '') => {
        const owner = linkedPack ?? currentPack;
        if (!ruleIdsByPack.has(owner)) return all;
        if (name !== 'pack' && !ruleIdsByPack.get(owner).has(name)) return all;
        const target = linkedPack ? `${up}${linkedPack}/` : fromPack;
        return `](${name === 'pack' ? target : `${target}${name}/`}${fragment})`;
      },
    );
  }

  rmSync(out, { recursive: true, force: true });
  const pages = new Map(packs.map((pack) => [pack, packPage(pack)]));

  for (const [pack, rules] of byPack) {
    const folder = join(out, pack);
    mkdirSync(folder, { recursive: true });
    const { title, body: packBody } = pages.get(pack);
    for (const rule of rules) {
      const versions = rule.since ? `${title} ${rule.since} and later` : 'every version';
      const body = [
        `| Rule | ${rule.rule} |`,
        '| --- | --- |',
        `| Pack | [${title}](../) |`,
        `| Severity | ${severityText(rule)} |`,
        `| Fix | ${fixText(rule)} |`,
        `| Library versions | ${versions} |`,
        '',
        siteLinks(rule.doc, pack, 2),
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
            editUrl: yaml(editUrl(`rules/${pack}/${rule.rule}.md`)),
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
    for (const { line, fenced } of parseMarkdown(packBody).lines) {
      const id = !fenced && line.match(/^- (\S+)$/)?.[1];
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
          editUrl: yaml(editUrl(`rules/${pack}/pack.md`)),
        },
        `${intro}\n\n${siteLinks(lines.join('\n'), pack, 1)}`,
      ),
    );
  }

  const overview = splitTitle(readFileSync(join(rulesFolder, 'README.md'), 'utf8'), 'rules/README.md').body;
  const table = [
    '| Pack | Rules | Safe fix | Unsafe fix | No fix |',
    '| --- | ---: | ---: | ---: | ---: |',
    ...packs.map((pack) => {
      const rules = byPack.get(pack);
      const count = (fix) => rules.filter((rule) => rule.fix === fix).length;
      return `| [${pages.get(pack).title}](./${pack}/) | ${rules.length} | ${count('safe')} | ${count('unsafe')} | ${count(null)} |`;
    }),
    `| All packs | ${catalog.length} | ${catalog.filter((r) => r.fix === 'safe').length} | ${catalog.filter((r) => r.fix === 'unsafe').length} | ${catalog.filter((r) => !r.fix).length} |`,
  ];
  writeFileSync(
    join(out, 'index.md'),
    page(
      {
        title: yaml('Rules'),
        description: yaml(`The ${catalog.length} rules that xpd checks, by pack.`),
        editUrl: yaml(editUrl('rules/README.md')),
      },
      `${overview.trim()}\n\n${table.join('\n')}`,
    ),
  );

  console.log(`rules.mjs: wrote ${catalog.length} rule pages and ${packs.length} pack pages`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  generateRules(buildXpd(repository));
}
