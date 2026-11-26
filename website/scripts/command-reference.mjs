import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

import { editUrl, page, sectionsText, yaml } from '../src/markdown.mjs';
import { cell, code, readAuthored, reflow, text, validateNames } from './reference-common.mjs';

const pagePath = (slug) => (slug === 'index' ? 'xpd-index' : slug);

function syntax(arg) {
  const names = arg.value_names?.length ? arg.value_names : [arg.id.toUpperCase()];
  const value = names.map((name) => `<${name}>`).join(' ');
  if (arg.positional) return `${value}${arg.multiple ? '...' : ''}`;
  const flags = [arg.short && `-${arg.short}`, arg.long && `--${arg.long}`].filter(Boolean).join(', ');
  if (!arg.takes_value) return flags;
  return arg.optional_value ? `${flags}[=${value}]` : `${flags} ${value}`;
}

function describeArg(arg) {
  const help = cell(arg.long_help ?? arg.help);
  const base = help ? (/[.!?:)]$/.test(help) ? help : `${help}.`) : '';
  const notes = [];
  if (arg.aliases?.length) notes.push(`Aliases: ${arg.aliases.map((alias) => code(`--${alias}`)).join(', ')}.`);
  if (arg.required) notes.push('Required.');
  if (arg.multiple && !arg.positional) notes.push('Repeatable.');
  if (arg.optional_value) notes.push(`Optional value via ${code(`--${arg.long}=<VALUE>`)}.`);
  if (arg.trailing) notes.push('Captures all following arguments.');
  if (arg.conflicts_with.length) notes.push(`Conflicts with ${arg.conflicts_with.map(code).join(', ')}.`);
  const values = arg.possible_values;
  if (values.length && values.some((value) => value.help)) {
    const described = values
      .map((value) => (value.help ? `${code(value.name)} (${cell(value.help)})` : code(value.name)))
      .join(', ');
    notes.push(`Values: ${described}.`);
  } else if (values.length) {
    notes.push(`Values: ${values.map((value) => code(value.name)).join(', ')}.`);
  }
  return [base, ...notes].filter(Boolean).join(' ');
}

function argumentTable(args) {
  const rows = args.map((arg) => `| ${code(syntax(arg))} | ${describeArg(arg)} |`);
  return ['| Argument | What it does |', '| --- | --- |', ...rows].join('\n');
}

function optionTable(options) {
  const rows = options.map((arg) => {
    const def = arg.takes_value && arg.defaults.length ? arg.defaults.map(code).join(', ') : ' ';
    return `| ${code(syntax(arg))} | ${def} | ${describeArg(arg)} |`;
  });
  return ['| Option | Default | What it does |', '| --- | --- | --- |', ...rows].join('\n');
}

export async function generateCommandReference(commands, { repository, commandFolder, commandsOut, serverHelp, version, checkExample }) {
  const bySlug = new Map(commands.map((command) => [command.name.split(' ').slice(1).join('/'), command]));
  validateNames(repository, 'command', commandFolder, ['README', ...bySlug.keys()]);
  rmSync(commandsOut, { recursive: true, force: true });

  for (const [slug, command] of bySlug) {
    const source = join(commandFolder, `${slug}.md`);
    const prose = readAuthored(source);
    const topLevel = !slug.includes('/');
    if (topLevel && !prose.sections.some((section) => section.heading === 'Exit status')) {
      throw new Error(`${relative(repository, source)} needs an "## Exit status" section`);
    }
    await checkExample(source, prose.source, command.name);

    const lead = prose.intro.trim() || text(command.about);
    const body = [lead, '', '## Usage', '', '```sh', command.usage, '```'];
    if (command.aliases.length) {
      body.push('', `Aliases: ${command.aliases.map((alias) => code(command.name.replace(/\S+$/, alias))).join(', ')}.`);
    }
    if (command.subcommand_required) body.push('', 'Pass one of the subcommands below.');
    if (command.subcommands.length) {
      body.push('', '## Subcommands', '', '| Subcommand | What it does |', '| --- | --- |');
      for (const name of command.subcommands) {
        const sub = bySlug.get(`${slug}/${name}`);
        body.push(`| [${code(sub.name)}](./${pagePath(`${slug}/${name}`).split('/').pop()}/) | ${cell(sub.about)} |`);
      }
    }
    const positional = command.arguments.filter((arg) => arg.positional);
    const options = command.arguments.filter((arg) => !arg.positional);
    if (positional.length) body.push('', '## Arguments', '', argumentTable(positional));
    if (options.length) body.push('', '## Options', '', optionTable(options));
    for (const group of command.groups) {
      const members = group.args.map((id) => command.arguments.find((arg) => arg.id === id)).filter(Boolean);
      body.push('', `Pass exactly one of ${members.map((arg) => code(`--${arg.long}`)).join(' and ')}.`);
    }
    if (command.name === 'xpd mcp') {
      body.push('', '## Server options', '', '```text', serverHelp.trim(), '```');
    }
    if (prose.sections.length) body.push('', sectionsText(prose.sections));

    const hasChildren = command.subcommands.length > 0;
    const file = join(commandsOut, hasChildren ? `${pagePath(slug)}/index.md` : `${pagePath(slug)}.md`);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(
      file,
      page(
        {
          title: yaml(command.name),
          description: yaml(command.about),
          editUrl: yaml(editUrl(`website/reference/commands/${slug}.md`)),
          sidebar: { label: yaml(slug.split('/').pop()), order: hasChildren ? 0 : 1 },
          tableOfContents: { maxHeadingLevel: 2 },
        },
        body.join('\n'),
      ),
    );
  }

  {
    const prose = readAuthored(join(commandFolder, 'README.md'));
    const topLevel = commands.filter((c) => c.name.split(' ').length === 2);
    const subSummary = topLevel
      .filter((c) => c.subcommands.length > 0)
      .map((c) => `, and ${code(c.name)} has ${c.subcommands.length} of its own`)
      .join('');
    const rows = [...bySlug].map(([slug, command]) => {
      const aliases = command.aliases.map((alias) => code(command.name.replace(/\S+$/, alias))).join(', ');
      return `| [${code(command.name)}](./${pagePath(slug)}/) | ${aliases} | ${cell(command.about)} |`;
    });
    writeFileSync(
      join(commandsOut, 'index.md'),
      page(
        {
          title: yaml('Commands'),
          description: yaml(`The ${bySlug.size} commands of xpd, with their arguments, options and exit status.`),
          editUrl: yaml(editUrl('website/reference/commands/README.md')),
          sidebar: { label: yaml('All commands'), order: 0 },
        },
        [
          prose.intro,
          `xpd ${version} has ${topLevel.length} commands${subSummary}.`,
          '',
          '| Command | Also called | What it does |',
          '| --- | --- | --- |',
          ...rows,
          '',
          sectionsText(prose.sections),
        ].join('\n'),
      ),
    );
  }
}
