// Writes the command reference into src/content/docs/commands/ and the MCP reference into
// src/content/docs/mcp/, from the xpd that this checkout builds and the MCP server it starts.
//
// The names, arguments, options, defaults, tools, resources and prompts come from the running
// tools: `xpd commands` prints the command tree that parses xpd's arguments, and
// scripts/mcp_listing.py starts `python -m xpd.mcp` and lists what it registers. When to use a
// command or a tool, worked examples and exit behavior are written by hand in reference/, one
// Markdown file for each command and tool. The build fails when a command or a tool has no such
// file, when a file names none, and when a file names a flag that its command does not take.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const website = join(dirname(fileURLToPath(import.meta.url)), '..');
const repository = join(website, '..');
const authored = join(website, 'reference');
const docs = join(website, 'src', 'content', 'docs');
const editBase = 'https://github.com/bjoernaagaard/xpd/edit/main/website/reference';
// The Python that runs the MCP server for the listing. Python 3.14 is the newest release that uv
// offers with the global interpreter lock on every platform it supports.
const venv = join(website, '.mcp-venv');
const PYTHON_VERSION = '3.14';

const run = (command, args, options = {}) =>
  execFileSync(command, args, {
    cwd: repository,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'inherit'],
    ...options,
  });

/** The path of the xpd binary that cargo builds from this checkout. */
function buildXpd() {
  const messages = run('cargo', ['build', '--quiet', '--message-format=json-render-diagnostics']);
  for (const line of messages.split('\n')) {
    if (!line.startsWith('{')) continue;
    const message = JSON.parse(line);
    if (message.reason === 'compiler-artifact' && message.target?.name === 'xpd' && message.executable) {
      return message.executable;
    }
  }
  throw new Error('cargo build named no xpd executable');
}

/** What the MCP server lists, from a Python environment with the server's dependencies. */
function mcpListing(xpd) {
  run('uv', ['venv', '--quiet', '--allow-existing', '--python', PYTHON_VERSION, venv]);
  run('uv', ['pip', 'install', '--quiet', '--python', venv, '-r', join(repository, 'pyproject.toml')]);
  const python = join(venv, process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python');
  return JSON.parse(
    run(python, [join(website, 'scripts', 'mcp_listing.py')], {
      env: { ...process.env, XPD_BIN: xpd, PYTHONPATH: join(repository, 'python') },
      stdio: ['ignore', 'pipe', 'ignore'],
    }),
  );
}

// Text from help and schemas goes into Markdown, where `<pack>` would read as an HTML tag and
// `*` as emphasis. Code spans keep their text as it is.
function text(value) {
  return String(value ?? '')
    .split(/(`[^`]*`)/)
    .map((part, index) =>
      index % 2 ? part : part.replace(/[\\*_[\]|]/g, '\\$&').replace(/</g, '&lt;').replace(/>/g, '&gt;'),
    )
    .join('');
}

/** Help text with its line breaks joined, keeping blank lines and list items. */
function reflow(value) {
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

const cell = (value) => reflow(value).replaceAll('\n', ' ');
const yaml = (value) => JSON.stringify(value);
const code = (value) => `\`${String(value).replaceAll('`', '')}\``;

function page(frontmatter, body) {
  const lines = Object.entries(frontmatter).map(([key, value]) =>
    typeof value === 'object' ? `${key}:\n${Object.entries(value).map(([k, v]) => `  ${k}: ${v}`).join('\n')}` : `${key}: ${value}`,
  );
  return `---\n${lines.join('\n')}\n---\n\n${body.trim()}\n`;
}

/** An authored file split into the text before its first `## ` heading and its sections. */
function readAuthored(path) {
  const source = readFileSync(path, 'utf8');
  const sections = [];
  let intro = [];
  let current = null;
  let fenced = false;
  for (const line of source.split('\n')) {
    if (/^\s*(```|~~~)/.test(line)) fenced = !fenced;
    if (!fenced && line.startsWith('## ')) {
      current = { heading: line.slice(3).trim(), lines: [] };
      sections.push(current);
    } else if (current) {
      current.lines.push(line);
    } else {
      intro.push(line);
    }
  }
  const body = (lines) => lines.join('\n').trim();
  return {
    source,
    intro: body(intro),
    sections: sections.map((section) => ({ heading: section.heading, body: body(section.lines) })),
  };
}

const sectionsText = (sections) =>
  sections.map((section) => `## ${section.heading}\n\n${section.body}`).join('\n\n');

/** Every Markdown file under `folder`, as paths relative to it without `.md`. */
function markdownFiles(folder) {
  if (!existsSync(folder)) return [];
  return readdirSync(folder, { recursive: true })
    .filter((file) => file.endsWith('.md'))
    .map((file) => file.replaceAll('\\', '/').replace(/\.md$/, ''));
}

/** Fails when the names of the authored files and the names of the listed items differ. */
function sameNames(what, folder, listed, files) {
  const missing = listed.filter((name) => !files.includes(name));
  const orphans = files.filter((name) => !listed.includes(name));
  if (missing.length || orphans.length) {
    throw new Error(
      `${relative(repository, folder)} needs one Markdown file for each ${what}. ` +
        `Missing: ${missing.join(', ') || 'none'}. Naming no ${what}: ${orphans.join(', ') || 'none'}.`,
    );
  }
}

// ---------------------------------------------------------------------------------------------
// Commands

const xpd = buildXpd();
const commands = JSON.parse(run(xpd, ['commands']));
const bySlug = new Map(commands.map((command) => [command.name.split(' ').slice(1).join('/'), command]));
const commandFolder = join(authored, 'commands');
// The page of a command, below commands/. Astro reads `index` as the page of its folder, which is
// the overview, so `xpd index` has the page xpd-index.
const pagePath = (slug) => (slug === 'index' ? 'xpd-index' : slug);
sameNames('command', commandFolder, ['README', ...bySlug.keys()], markdownFiles(commandFolder));

/** How a reader writes an argument, such as `-c, --config <CONFIG>` or `<PATHS>...`. */
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
  // clap drops the full stop at the end of a one-line help.
  const help = reflow(arg.long_help ?? arg.help);
  const lines = [`### ${text(heading)}`, '', code(syntax(arg)), '', /[.!?:)]$/.test(help) ? help : `${help}.`];
  const facts = [];
  if (arg.aliases?.length) facts.push(`Also written ${arg.aliases.map((alias) => code(`--${alias}`)).join(', ')}.`);
  if (arg.required) facts.push('Required.');
  // A flag without a value has the default false, which says nothing.
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

/** The long options of each command, to check the flags that authored prose names. */
const flagsOf = new Map(
  commands.map((command) => [
    command.name,
    new Set(command.arguments.flatMap((arg) => [arg.long, ...(arg.aliases ?? [])].filter(Boolean).map((name) => `--${name}`))),
  ]),
);
const canonical = new Map(commands.flatMap((command) => [command.name, ...command.aliases.map((alias) => command.name.replace(/\S+$/, alias))].map((name) => [name, command.name])));
const builtinFlags = new Set(['--help', '--version']);

/**
 * The flags in `xpd ...` commands of an authored file that their command does not take. A
 * command shows as inline code or as a line of a code block, such as `xpd check --fix` or
 * `$ xpd run -p '$X' --json`. A bare `--flag` in inline code must belong to `pageCommand`.
 */
function unknownFlags(source, pageCommand) {
  const problems = [];
  const check = (snippet, context) => {
    const words = snippet.replace(/^\$\s+/, '').replace(/^uvx\s+/, '').split(/\s+/);
    if (words[0] !== 'xpd') return;
    let name = 'xpd';
    for (const word of words.slice(1)) {
      const longer = `${name} ${word}`;
      if (!canonical.has(longer)) break;
      name = longer;
    }
    const command = canonical.get(name);
    if (!command) return;
    for (const word of words) {
      const flag = word.match(/^(--[a-z][a-z0-9-]*)/)?.[1];
      if (flag && !builtinFlags.has(flag) && !flagsOf.get(command).has(flag) && command !== 'xpd mcp') {
        problems.push(`${flag} in ${context}`);
      }
    }
  };
  let fenced = false;
  for (const line of source.split('\n')) {
    if (/^\s*(```|~~~)/.test(line)) {
      fenced = !fenced;
      continue;
    }
    if (fenced) {
      check(line.trim(), `"${line.trim()}"`);
      continue;
    }
    for (const [, span] of line.matchAll(/`([^`]+)`/g)) {
      if (span.startsWith('xpd ') || span.startsWith('uvx xpd ')) check(span, `\`${span}\``);
      else if (pageCommand && /^--[a-z][a-z0-9-]*$/.test(span.split(/[ =]/)[0])) {
        const flag = span.split(/[ =]/)[0];
        if (!builtinFlags.has(flag) && !flagsOf.get(pageCommand).has(flag)) problems.push(`${flag} in \`${span}\``);
      }
    }
  }
  return problems;
}

const commandsOut = join(docs, 'commands');
rmSync(commandsOut, { recursive: true, force: true });
const flagProblems = [];
const mcp = mcpListing(xpd);

for (const [slug, command] of bySlug) {
  const source = join(commandFolder, `${slug}.md`);
  const prose = readAuthored(source);
  const topLevel = !slug.includes('/');
  if (topLevel && !prose.sections.some((section) => section.heading === 'Exit status')) {
    throw new Error(`${relative(repository, source)} needs an "## Exit status" section`);
  }
  flagProblems.push(...unknownFlags(prose.source, command.name).map((problem) => `${relative(repository, source)}: ${problem}`));

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
    body.push('', '## Server options', '', `${code('xpd mcp')} passes its arguments to the server, whose help lists these flags.`, '', '```text', mcp.help.trim(), '```');
  }
  if (!options.length && !positional.length && !command.subcommands.length) body.push('', 'It takes no arguments.');
  body.push('', sectionsText(prose.sections));

  // A command with commands of its own gets a folder.
  const hasChildren = command.subcommands.length > 0;
  const file = join(commandsOut, hasChildren ? `${pagePath(slug)}/index.md` : `${pagePath(slug)}.md`);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(
    file,
    page(
      {
        title: yaml(command.name),
        description: yaml(command.about),
        editUrl: yaml(`${editBase}/commands/${slug}.md`),
        sidebar: { label: yaml(slug.split('/').pop()), order: hasChildren ? 0 : 1 },
        tableOfContents: { maxHeadingLevel: 2 },
      },
      body.join('\n'),
    ),
  );
}

{
  const prose = readAuthored(join(commandFolder, 'README.md'));
  flagProblems.push(...unknownFlags(prose.source, null).map((problem) => `website/reference/commands/README.md: ${problem}`));
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
        editUrl: yaml(`${editBase}/commands/README.md`),
        sidebar: { label: yaml('All commands'), order: 0 },
      },
      [
        prose.intro,
        `xpd ${mcp.server.version} has ${commands.filter((c) => c.name.split(' ').length === 2).length} commands, and ${code('xpd new')} has ${bySlug.get('new').subcommands.length} of its own.`,
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

// ---------------------------------------------------------------------------------------------
// MCP server

/** A JSON schema's type in words, such as `list of string` or `"error", "warning" or null`. */
function typeOf(schema) {
  if (!schema) return 'any';
  if (schema.anyOf) {
    const kinds = schema.anyOf.map(typeOf);
    return [...new Set(kinds)].join(' or ');
  }
  if (schema.enum) return schema.enum.map((value) => code(JSON.stringify(value))).join(', ');
  if (schema.const !== undefined) return code(JSON.stringify(schema.const));
  if (schema.type === 'array') return `list of ${typeOf(schema.items)}`;
  if (Array.isArray(schema.type)) return schema.type.join(' or ');
  return schema.type ?? 'any';
}

/** The object schemas inside `schema`, through `anyOf` and arrays. */
function objectsIn(schema) {
  if (!schema) return [];
  if (schema.anyOf) return schema.anyOf.flatMap(objectsIn);
  if (schema.type === 'array') return objectsIn(schema.items);
  return schema.properties ? [schema] : [];
}

/** One row for each field inside `schema`, with its path from the argument, such as `options.rules`. */
function fieldRows(schema, prefix) {
  const rows = [];
  for (const object of objectsIn(schema)) {
    const list = schema.type === 'array' || schema.anyOf?.some((alt) => alt.type === 'array');
    const base = list ? `${prefix}[]` : prefix;
    const required = new Set(object.required ?? []);
    for (const [name, field] of Object.entries(object.properties)) {
      const path = `${base}.${name}`;
      rows.push({ path, field, required: required.has(name) });
      rows.push(...fieldRows(field, path));
    }
  }
  return rows;
}

function fieldTable(rows, summary) {
  const table = [
    '| Field | Type | Required | Default | Description |',
    '| --- | --- | --- | --- | --- |',
    ...rows.map(
      ({ path, field, required }) =>
        `| ${code(path)} | ${cell(typeOf(field))} | ${required ? 'yes' : 'no'} | ${'default' in field ? code(JSON.stringify(field.default)) : ' '} | ${cell(field.description) || ' '} |`,
    ),
  ];
  // A long table folds away, and search still finds the fields in it.
  return rows.length > 12 ? `<details>\n<summary>${summary}</summary>\n\n${table.join('\n')}\n\n</details>` : table.join('\n');
}

/** How a call names the arguments, such as `check(paths=null, unsafe_fixes=false)`. */
function signature(tool) {
  const schema = tool.inputSchema;
  const required = new Set(schema.required ?? []);
  const args = Object.entries(schema.properties ?? {}).map(([name, field]) =>
    required.has(name) ? name : `${name}=${JSON.stringify(field.default ?? null)}`,
  );
  return `${tool.name}(${args.join(', ')})`;
}

const BEHAVIOR = [
  ['readOnlyHint', 'Changes no file', 'May change files'],
  ['destructiveHint', 'May overwrite what is there', 'Only adds to what is there'],
  ['idempotentHint', 'A second call with the same arguments changes nothing more', 'A second call may do more'],
  ['openWorldHint', 'Runs code that may reach outside the project', 'Stays inside the project'],
];

function toolPage(tool, prose) {
  const schema = tool.inputSchema;
  const required = new Set(schema.required ?? []);
  const body = [prose.intro, '## Call', '', '```python', signature(tool), '```'];
  const annotations = tool.annotations ?? {};
  const behavior = BEHAVIOR.filter(([key]) => key in annotations).map(([key, yes, no]) => (annotations[key] ? yes : no));
  if (annotations.readOnlyHint) behavior.splice(1, 1);
  if (behavior.length) body.push('', ...behavior.map((line) => `- ${line}`));
  const tags = tool._meta?.fastmcp?.tags ?? [];
  if (tags.length) body.push('', `Tags: ${tags.map(code).join(', ')}.`);
  body.push('', '## Description', '', 'The server gives the agent this description of the tool.', '', reflow(tool.description));
  const args = Object.entries(schema.properties ?? {});
  body.push('', '## Arguments', '');
  if (!args.length) body.push('It takes no arguments.');
  for (const [name, field] of args) {
    body.push(`### ${text(name)}`, '', `Type: ${typeOf(field)}. ${required.has(name) ? 'Required.' : `Optional, default ${code(JSON.stringify(field.default ?? null))}.`}`);
    if (field.description) body.push('', reflow(field.description));
    const rows = fieldRows(field, name);
    if (rows.length) body.push('', fieldTable(rows, `The ${rows.length} fields of ${code(name)}`));
    body.push('');
  }
  body.push('## Result', '');
  if (tool.outputSchema) {
    if (tool.outputSchema.description) body.push(reflow(tool.outputSchema.description), '');
    const rows = fieldRows(tool.outputSchema, 'result').map((row) => ({ ...row, path: row.path.replace(/^result\.?/, '') }));
    body.push(rows.length ? fieldTable(rows, `The ${rows.length} fields of the result`) : `Type: ${typeOf(tool.outputSchema)}.`);
  } else {
    body.push('The tool declares no output schema.');
  }
  body.push('', sectionsText(prose.sections));
  return body.join('\n');
}

const mcpFolder = join(authored, 'mcp');
const mcpOut = join(docs, 'mcp');
rmSync(mcpOut, { recursive: true, force: true });
mkdirSync(join(mcpOut, 'tools'), { recursive: true });
const toolNames = mcp.tools.map((tool) => tool.name);
sameNames('tool', join(mcpFolder, 'tools'), toolNames, markdownFiles(join(mcpFolder, 'tools')));
sameNames('page', mcpFolder, ['index', 'resources', 'prompts', ...toolNames.map((name) => `tools/${name}`)], markdownFiles(mcpFolder));

const firstSentence = (value) => (reflow(value).split('\n')[0].match(/^.*?[.!?](?=\s|$)/)?.[0] ?? reflow(value).split('\n')[0]);

for (const tool of mcp.tools) {
  const source = join(mcpFolder, 'tools', `${tool.name}.md`);
  const prose = readAuthored(source);
  if (!prose.sections.some((section) => section.heading === 'Errors')) {
    throw new Error(`${relative(repository, source)} needs an "## Errors" section`);
  }
  flagProblems.push(...unknownFlags(prose.source, null).map((problem) => `${relative(repository, source)}: ${problem}`));
  writeFileSync(
    join(mcpOut, 'tools', `${tool.name}.md`),
    page(
      {
        title: yaml(`${tool.name} tool`),
        description: yaml(`The ${tool.name} tool of the xpd MCP server. ${firstSentence(tool.description).replaceAll('\\', '')}`),
        editUrl: yaml(`${editBase}/mcp/tools/${tool.name}.md`),
        sidebar: { label: yaml(tool.name) },
        tableOfContents: { maxHeadingLevel: 3 },
      },
      toolPage(tool, prose),
    ),
  );
}

/** A section for each named item, with the authored text whose heading is its name. */
function namedSections(what, source, items, render) {
  const prose = readAuthored(source);
  const names = items.map((item) => item.key);
  const headings = prose.sections.map((section) => section.heading);
  const extra = headings.filter((heading) => !names.includes(heading) && !heading.startsWith('About '));
  const missing = names.filter((name) => !headings.includes(name));
  if (missing.length || extra.length) {
    throw new Error(
      `${relative(repository, source)} needs a "## <name>" section for each ${what}, and "## About ..." for others. ` +
        `Missing: ${missing.join(', ') || 'none'}. Naming no ${what}: ${extra.join(', ') || 'none'}.`,
    );
  }
  flagProblems.push(...unknownFlags(prose.source, null).map((problem) => `${relative(repository, source)}: ${problem}`));
  const byHeading = new Map(prose.sections.map((section) => [section.heading, section.body]));
  const sections = items.map((item) => [`## ${item.heading}`, '', render(item), '', byHeading.get(item.key)].join('\n'));
  const about = prose.sections.filter((section) => section.heading.startsWith('About ')).map((section) => `## ${section.heading.slice(6)}\n\n${section.body}`);
  return [prose.intro, ...sections, ...about].join('\n\n');
}

// Resources: a section for each fixed resource and template. The skills add one resource for each
// of their files and a manifest, and the views add their renderer, so those come in groups.
{
  const groupOf = (uri) =>
    uri.startsWith('skill://') ? `skill://${uri.split('/')[2]}/` : uri.startsWith('ui://') ? 'ui://' : uri;
  const groups = new Map();
  for (const resource of mcp.resources) {
    const key = groupOf(resource.uri);
    groups.set(key, [...(groups.get(key) ?? []), resource]);
  }
  const resourceItems = [...groups].map(([key, resources]) => ({ key, heading: text(key), resources, template: false }));
  const templateItems = mcp.resource_templates.map((template) => ({ key: template.uriTemplate, heading: text(template.uriTemplate), resources: [template], template: true }));
  const render = (item) => {
    const lines = [];
    if (item.template) lines.push('A resource template. Fill in the parts in braces to read one resource.', '');
    lines.push('| URI | Name | Type | Description |', '| --- | --- | --- | --- |');
    for (const resource of item.resources) {
      lines.push(`| ${code(resource.uri ?? resource.uriTemplate)} | ${text(resource.name)} | ${code(resource.mimeType ?? 'unknown')} | ${cell(resource.description) || ' '} |`);
    }
    const audience = item.resources[0].annotations?.audience;
    if (audience) lines.push('', `Meant for the ${audience.map((who) => (who === 'assistant' ? 'agent' : who)).join(' and the ')}.`);
    return lines.join('\n');
  };
  const body = namedSections('resource or template', join(mcpFolder, 'resources.md'), [...resourceItems, ...templateItems], render);
  writeFileSync(
    join(mcpOut, 'resources.md'),
    page(
      {
        title: yaml('MCP resources'),
        description: yaml(`The ${mcp.resources.length} resources and ${mcp.resource_templates.length} resource templates of the xpd MCP server.`),
        editUrl: yaml(`${editBase}/mcp/resources.md`),
        sidebar: { label: yaml('Resources'), order: 1 },
      },
      `The server lists ${mcp.resources.length} resources and ${mcp.resource_templates.length} resource templates.\n\n${body}`,
    ),
  );
}

// Prompts: a section for each prompt with its arguments.
{
  const items = mcp.prompts.map((prompt) => ({ key: prompt.name, heading: text(prompt.name), prompt }));
  const render = ({ prompt }) => {
    const lines = [reflow(prompt.description), ''];
    const args = prompt.arguments ?? [];
    if (args.length) {
      lines.push('| Argument | Required | Description |', '| --- | --- | --- |');
      lines.push(...args.map((arg) => `| ${code(arg.name)} | ${arg.required ? 'yes' : 'no'} | ${cell(arg.description) || ' '} |`));
    } else {
      lines.push('It takes no arguments.');
    }
    return lines.join('\n');
  };
  const body = namedSections('prompt', join(mcpFolder, 'prompts.md'), items, render);
  writeFileSync(
    join(mcpOut, 'prompts.md'),
    page(
      {
        title: yaml('MCP prompts'),
        description: yaml(`The ${mcp.prompts.length} prompts of the xpd MCP server, with their arguments.`),
        editUrl: yaml(`${editBase}/mcp/prompts.md`),
        sidebar: { label: yaml('Prompts'), order: 2 },
      },
      `The server lists ${mcp.prompts.length} prompts.\n\n${body}`,
    ),
  );
}

// The overview: what the server lists, the tools in code mode, and the server's instructions.
{
  const prose = readAuthored(join(mcpFolder, 'index.md'));
  flagProblems.push(...unknownFlags(prose.source, null).map((problem) => `website/reference/mcp/index.md: ${problem}`));
  const behavior = (tool) => (tool.annotations?.readOnlyHint ? 'No' : 'Yes');
  const body = [
    prose.intro,
    '',
    `The server is named ${code(mcp.server.name)}, and xpd ${mcp.server.version} lists ${mcp.tools.length} tools, ${mcp.resources.length} resources, ${mcp.resource_templates.length} resource templates and ${mcp.prompts.length} prompts.`,
    '',
    '## Tools',
    '',
    '| Tool | What it does | May change files |',
    '| --- | --- | --- |',
    ...mcp.tools.map((tool) => `| [${code(tool.name)}](./tools/${tool.name}/) | ${firstSentence(tool.description)} | ${behavior(tool)} |`),
    '',
    'The [resources](./resources/) page lists the resources and templates, and the [prompts](./prompts/) page lists the prompts.',
    '',
    '## Code mode',
    '',
    `With ${code('--code-mode')}, the server lists these ${mcp.code_mode_tools.length} tools in place of the ones above.`,
    '',
    '| Tool | Arguments | Description |',
    '| --- | --- | --- |',
    ...mcp.code_mode_tools.map((tool) => {
      const required = new Set(tool.inputSchema.required ?? []);
      const args = Object.keys(tool.inputSchema.properties ?? {}).map((name) => code(required.has(name) ? name : `${name}?`));
      return `| ${code(tool.name)} | ${args.join(', ') || 'none'} | ${cell(tool.description)} |`;
    }),
    '',
    '## Instructions',
    '',
    'The server gives the agent these instructions when it connects.',
    '',
    '```text',
    mcp.instructions.trim(),
    '```',
    '',
    sectionsText(prose.sections),
  ];
  writeFileSync(
    join(mcpOut, 'index.md'),
    page(
      {
        title: yaml('MCP server reference'),
        description: yaml(`The tools, resources and prompts of the xpd MCP server, as xpd ${mcp.server.version} lists them.`),
        editUrl: yaml(`${editBase}/mcp/index.md`),
        sidebar: { label: yaml('Overview'), order: 0 },
      },
      body.join('\n'),
    ),
  );
}

if (flagProblems.length) {
  throw new Error(`Authored reference pages name flags that their command does not take:\n${flagProblems.join('\n')}`);
}

console.log(
  `reference.mjs: wrote ${bySlug.size} command pages and ${mcp.tools.length} tool pages, with ` +
    `${mcp.resources.length} resources, ${mcp.resource_templates.length} templates and ${mcp.prompts.length} prompts`,
);
