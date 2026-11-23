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

function argumentSection(arg) {
  const heading = arg.positional ? `<${(arg.value_names?.[0] ?? arg.id).toUpperCase()}>` : `--${arg.long ?? arg.short}`;
  const help = reflow(arg.long_help ?? arg.help);
  const lines = [`### ${text(heading)}`, '', code(syntax(arg)), '', /[.!?:)]$/.test(help) ? help : `${help}.`];
  const facts = [];
  if (arg.aliases?.length) facts.push(`Also written ${arg.aliases.map((alias) => code(`--${alias}`)).join(', ')}.`);
  if (arg.required) facts.push('Required.');
  if (arg.takes_value && arg.defaults.length) facts.push(`Default: ${arg.defaults.map(code).join(', ')}.`);
  if (arg.multiple && !arg.positional) facts.push('Give it more than once to pass several values.');
  if (arg.optional_value) facts.push(`The value is optional. Give it as ${code(`--${arg.long}=<VALUE>`)}.`);
  if (arg.trailing) facts.push('Every argument after it goes to it, flags included.');
  if (arg.conflicts_with.length) facts.push(`Cannot be combined with ${arg.conflicts_with.map(code).join(', ')}.`);
  if (facts.length) lines.push('', facts.join(' '));
  const values = arg.possible_values;
  if (values.length && values.some((value) => value.help)) {
    lines.push('', '| Value | Meaning |', '| --- | --- |', ...values.map((value) => `| ${code(value.name)} | ${cell(value.help) || ' '} |`));
  } else if (values.length) {
    lines.push('', `Values: ${values.map((value) => code(value.name)).join(', ')}.`);
  }
  return lines.join('\n');
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

    const body = [text(command.about), prose.intro, '## Usage', '', '```sh', command.usage, '```'];
    if (command.aliases.length) {
      body.push('', `Also called ${command.aliases.map((alias) => code(command.name.replace(/\S+$/, alias))).join(', ')}.`);
    }
    if (command.subcommand_required) body.push('', 'It needs one of the commands below.');
    if (command.after_help) body.push('', `${code(`${command.name} --help`)} ends with these lines.`, '', '```text', command.after_help.trim(), '```');
    if (command.subcommands.length) {
      body.push('', '## Commands', '', '| Command | What it does |', '| --- | --- |');
      for (const name of command.subcommands) {
        const sub = bySlug.get(`${slug}/${name}`);
        body.push(`| [${code(sub.name)}](./${pagePath(`${slug}/${name}`).split('/').pop()}/) | ${cell(sub.about)} |`);
      }
    }
    const positional = command.arguments.filter((arg) => arg.positional);
    const options = command.arguments.filter((arg) => !arg.positional);
    if (positional.length) body.push('', '## Arguments', '', ...positional.map((arg) => `${argumentSection(arg)}\n`));
    if (options.length) body.push('', '## Options', '', ...options.map((arg) => `${argumentSection(arg)}\n`));
    for (const group of command.groups) {
      const members = group.args.map((id) => command.arguments.find((arg) => arg.id === id)).filter(Boolean);
      body.push('', `Give exactly one of ${members.map((arg) => code(`--${arg.long}`)).join(' and ')}.`);
    }
    if (command.name === 'xpd mcp') {
      body.push('', '## Server options', '', `${code('xpd mcp')} passes its arguments to the server, whose help lists these flags.`, '', '```text', serverHelp.trim(), '```');
    }
    if (!options.length && !positional.length && !command.subcommands.length) body.push('', 'It takes no arguments.');
    body.push('', sectionsText(prose.sections));

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
