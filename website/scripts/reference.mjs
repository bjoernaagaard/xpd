
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { parseMarkdown } from '../src/markdown.mjs';
import { exampleRunner } from './examples.mjs';
import { generateMcpReference } from './mcp-reference.mjs';
import { generateCommandReference } from './command-reference.mjs';
import { buildXpd, run as exec } from './exec.mjs';

const website = join(dirname(fileURLToPath(import.meta.url)), '..');
const repository = join(website, '..');
const authored = join(website, 'reference');
const docs = join(website, 'src', 'content', 'docs');
// The Python that runs the MCP server for the listing. Python 3.14 is the newest release that uv
// offers with the global interpreter lock on every platform it supports.
const venv = join(website, '.mcp-venv');
const PYTHON_VERSION = '3.14';

const run = (command, args, options = {}) => exec(command, args, { cwd: repository, ...options });

// The libraries that the scripts in samples/ import, which the examples of `check`, `equiv`,
// `profile` and `refit` read or run. The pins keep an example's output the same from build to
// build. The weekly build sets XPD_NEWEST_LIBRARIES, which drops the pins, so it shows when a new
// release of a library changes what an example prints.
const SAMPLE_LIBRARIES = ['pandas==3.1.1', 'pyarrow==26.0.1', 'scikit-learn==1.9.1', 'joblib==1.6.0', 'numpy==2.5.5'].map((pin) =>
  process.env.XPD_NEWEST_LIBRARIES ? pin.split('==')[0] : pin,
);

/** The Python of an environment with the MCP server's dependencies and the samples' libraries. */
function serverPython() {
  run('uv', ['venv', '--quiet', '--allow-existing', '--python', PYTHON_VERSION, venv]);
  run('uv', ['pip', 'install', '--quiet', '--upgrade', '--python', venv, '-r', join(repository, 'pyproject.toml'), ...SAMPLE_LIBRARIES]);
  return join(venv, process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python');
}

/** What the MCP server lists. */
function mcpListing(xpd, python) {
  return JSON.parse(
    run(python, [join(website, 'scripts', 'mcp_listing.py')], {
      env: { ...process.env, XPD_BIN: xpd, PYTHONPATH: join(repository, 'python') },
      stdio: ['ignore', 'pipe', 'ignore'],
    }),
  );
}

// Why the page of a command or a tool runs no example of it.
const NO_EXAMPLE = {
  'xpd lsp': 'It speaks the Language Server Protocol on stdin, which no shell line shows.',
  animate_render: 'It needs the pinned Manim image and rootless Podman.',
  animate_artifact: 'It needs an artifact from a completed local render.',
  animate_source: 'It needs a saved source bundle from a completed local render.',
  gates: 'Its arguments are the full results of six other tools.',
  discovery_gate: 'Its arguments hold a discovery record, which is 28 kB for samples/pipeline.',
};

export async function generateReference(xpd) {
  const python = serverPython();
  const examples = exampleRunner({ repository, xpd, python, pythonPath: join(repository, 'python') });
  const commands = JSON.parse(run(xpd, ['commands']));
  const commandFolder = join(authored, 'commands');

  const flagsOf = new Map(
    commands.map((command) => [
      command.name,
      new Set(command.arguments.flatMap((arg) => [arg.long, ...(arg.aliases ?? [])].filter(Boolean).map((name) => `--${name}`))),
    ]),
  );
  const canonical = new Map(commands.flatMap((command) => [command.name, ...command.aliases.map((alias) => command.name.replace(/\S+$/, alias))].map((name) => [name, command.name])));
  async function runExamples(source, name, options) {
    const file = relative(repository, source);
    const page = await examples.check(file, NO_EXAMPLE[name] ? {} : options);
    const spellings = [...canonical].filter(([, command]) => command === name).map(([spelling]) => new RegExp(`\\b${spelling}\\b`));
    const ranIt = options?.tool
      ? page.tools.includes(options.tool)
      : page.commands.some((command) => spellings.some((spelling) => spelling.test(command)));
    if (ranIt === Boolean(NO_EXAMPLE[name])) {
      examples.report(ranIt ? `${file} runs an example of ${name}, so take ${name} out of NO_EXAMPLE` : `${file} needs an example that runs ${name}`);
    }
  }

  const builtinFlags = new Set(['--help', '--version']);

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
    for (const { line, fenced, isFence } of parseMarkdown(source).lines) {
      if (isFence) continue;
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

  const flagProblems = [];
  const mcp = mcpListing(xpd, python);
  const checkFlags = (source, markdown, name) => {
    const file = relative(repository, source);
    flagProblems.push(...unknownFlags(markdown, name).map((problem) => `${file}: ${problem}`));
  };
  await generateCommandReference(commands, {
    repository,
    commandFolder,
    commandsOut: join(docs, 'commands'),
    serverHelp: mcp.help,
    version: run(xpd, ['--version']).trim().replace(/^xpd /, ''),
    checkExample: async (source, markdown, name) => {
      checkFlags(source, markdown, name);
      await runExamples(source, name);
    },
  });

  const mcpFolder = join(authored, 'mcp');
  await generateMcpReference(mcp, {
    repository,
    mcpFolder,
    mcpOut: join(docs, 'mcp'),
    checkExample: async (source, markdown, name) => {
      checkFlags(source, markdown, null);
      await runExamples(source, name, { tool: name });
    },
  });

  function docFiles(folder) {
    if (!existsSync(folder)) return [];
    return readdirSync(folder, { recursive: true })
      .filter((file) => /\.mdx?$/.test(file))
      .map((file) => join(folder, file));
  }

  const guides = join(docs, 'guides');
  const skills = join(repository, 'skills');
  const otherPages = [join(repository, 'README.md'), join(docs, 'index.md'), join(commandFolder, 'README.md'), ...['index', 'resources', 'prompts'].map((name) => join(mcpFolder, `${name}.md`)), ...docFiles(guides), ...docFiles(skills)];
  for (const source of otherPages) {
    const file = relative(repository, source);
    flagProblems.push(...unknownFlags(readFileSync(source, 'utf8'), null).map((problem) => `${file}: ${problem}`));
    await examples.check(file);
  }

  if (flagProblems.length) {
    throw new Error(`Pages name flags that their command does not take:\n${flagProblems.join('\n')}`);
  }

  const ran = await examples.finish();

  console.log(
    `reference.mjs: ran ${ran.commands.length} commands and ${ran.tools.length} tool calls of the examples, and wrote ${commands.length} command pages and ${mcp.tools.length} tool pages, with ` +
      `${mcp.resources.length} resources, ${mcp.resource_templates.length} templates and ${mcp.prompts.length} prompts`,
  );
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await generateReference(buildXpd(repository));
}
