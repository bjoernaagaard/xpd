// Runs the examples that the pages show, with the xpd that this checkout builds, and reports each
// example whose real output differs from the page.
//
// A page is one session. It starts in a new folder, shown on the pages as /tmp/project, that holds
// a copy of samples/ in which samples/pipeline/train.py has written its model. The code blocks of
// the page then take effect from top to bottom.
//
// - A block with `title="<path>"` is a file of the session, written before the blocks below it.
// - A `console` block is a shell session. A line that starts with `$ ` is a command, and the lines
//   below it are what the command prints, to stdout and stderr together.
// - On the page of an MCP tool, the `json` blocks of a section named Example come in pairs, the
//   arguments of a call and then its result. The server's root is the session folder, or the
//   folder below it that the arguments block names with `root=<folder>`. The server answers no to
//   every question that a tool asks the user.
//
// A page shows only real output, and it may leave some out. In a console block, a line that holds
// only `...` stands for any number of lines. In a result, an object may leave keys out, the string
// "..." stands for any value, and in a list it stands for any number of items.
//
// `XPD_EXAMPLES=write npm run reference` writes what xpd prints into each console block that
// differs, so a change of xpd's output is one command and a review of the diff.

import { execFileSync, spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scripts = dirname(fileURLToPath(import.meta.url));
const SHOWN_ROOT = '/tmp/project';
const ANY = '...';

/** The fenced code blocks of a Markdown source, each with the heading of its `## ` section. */
function blocks(source) {
  const found = [];
  let open = null;
  let section = '';
  for (const [index, line] of source.split('\n').entries()) {
    if (open) {
      if (line.trimEnd() === open.fence) {
        found.push(open);
        open = null;
      } else {
        open.lines.push(line);
      }
      continue;
    }
    const start = line.match(/^(```+|~~~+)\s*([\w-]*)\s*(.*)$/);
    if (start) {
      open = { fence: start[1], language: start[2], meta: start[3], lines: [], line: index + 1, section };
    } else if (line.startsWith('## ')) {
      section = line.slice(3).trim();
    }
  }
  return found;
}

const metaValue = (meta, key) => meta.match(new RegExp(`\\b${key}=(?:"([^"]*)"|(\\S+))`))?.slice(1).find(Boolean);

/** The commands of a console block, each with the output lines that the page shows below it. */
function consoleCases(block) {
  const cases = [];
  for (const line of block.lines) {
    if (line.startsWith('$ ')) cases.push({ command: line.slice(2), shown: [] });
    else if (cases.length) cases.at(-1).shown.push(line);
  }
  return cases;
}

const trimmed = (lines) => {
  const kept = lines.map((line) => line.trimEnd());
  while (kept.length && !kept.at(-1)) kept.pop();
  while (kept.length && !kept[0]) kept.shift();
  return kept;
};

/** Whether the real lines are the shown lines, where a shown `...` is any number of real lines. */
function linesMatch(shown, real) {
  const reach = new Set([0]);
  for (const line of shown) {
    const next = new Set();
    for (const at of reach) {
      if (line === ANY) for (let to = at; to <= real.length; to += 1) next.add(to);
      else if (real[at] === line) next.add(at + 1);
    }
    if (!next.size) return false;
    reach.clear();
    for (const at of next) reach.add(at);
  }
  return reach.has(real.length);
}

/** Whether the real value holds what the page shows of it. */
function valueMatches(shown, real) {
  if (shown === ANY) return true;
  if (Array.isArray(shown)) {
    if (!Array.isArray(real)) return false;
    const reach = new Set([0]);
    for (const item of shown) {
      const next = new Set();
      for (const at of reach) {
        if (item === ANY) for (let to = at; to <= real.length; to += 1) next.add(to);
        else if (at < real.length && valueMatches(item, real[at])) next.add(at + 1);
      }
      if (!next.size) return false;
      reach.clear();
      for (const at of next) reach.add(at);
    }
    return reach.has(real.length);
  }
  if (shown && typeof shown === 'object') {
    return Boolean(real) && typeof real === 'object' && !Array.isArray(real) && Object.entries(shown).every(([key, value]) => key in real && valueMatches(value, real[key]));
  }
  return shown === real;
}

/** What a tool call returned: its JSON, or `{"error": "<message>"}` when the call failed. */
function answerValue(text, failed) {
  if (failed) return { error: text };
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

/**
 * Runs the examples of pages. `xpd` is the binary, `python` the interpreter of an environment
 * that holds the MCP server's dependencies and the libraries that the samples import, and
 * `pythonPath` the folder of the `xpd` Python package.
 */
export function exampleRunner({ repository, xpd, python, pythonPath }) {
  const work = mkdtempSync(join(tmpdir(), 'xpd-examples-'));
  // The session folders go when the build ends, also when it fails before `finish`.
  process.on('exit', () => rmSync(work, { recursive: true, force: true }));
  const home = join(work, 'home');
  mkdirSync(home);
  // `xpd find` keeps its search model here, and a new download for each build is slow.
  const models = join(process.env.XDG_CACHE_HOME || join(homedir(), '.cache'), 'xpd', 'models');
  mkdirSync(models, { recursive: true });
  const env = {
    PATH: [dirname(xpd), dirname(python), '/usr/local/bin', '/usr/bin', '/bin'].join(':'),
    // An empty home, so the settings of the person who builds the site change no example.
    HOME: home,
    VIRTUAL_ENV: dirname(dirname(python)),
    XPD_BIN: xpd,
    PYTHONPATH: pythonPath,
    NO_COLOR: '1',
    COLUMNS: '100',
    LANG: 'C.UTF-8',
  };
  // xpd keeps what a hook session changed in its cache folder, so each session gets a new one,
  // with a link to the search model.
  const sessionEnv = (root) => {
    const cache = `${root}-cache`;
    mkdirSync(join(cache, 'xpd'), { recursive: true });
    symlinkSync(models, join(cache, 'xpd', 'models'));
    return { ...env, XDG_CACHE_HOME: cache };
  };
  const template = join(work, 'template');
  cpSync(join(repository, 'samples'), join(template, 'samples'), { recursive: true });
  const setup = { env: sessionEnv(template), stdio: ['ignore', 'ignore', 'inherit'] };
  execFileSync(python, ['train.py'], { cwd: join(template, 'samples', 'pipeline'), ...setup });
  // The first search downloads the model when the cache has none, and says so on stderr.
  execFileSync(xpd, ['find', 'rolling mean', '--limit', '1'], { cwd: template, ...setup });

  const problems = [];
  const write = process.env.XPD_EXAMPLES === 'write';
  const written = new Set();
  const ran = { commands: [], tools: [] };
  let sessions = 0;
  const shown = (root, value) => value.replaceAll(root, SHOWN_ROOT);

  /** Runs one console block in a shell that starts in the session folder. */
  function runConsole(root, env, cases) {
    const mark = `xpd-example-${process.pid}`;
    const script = cases
      .map(({ command }, index) => `${command.replaceAll(SHOWN_ROOT, root)}\n__status=$?; printf '\\n${mark} ${index}\\n'; (exit $__status)`)
      .join('\n');
    const result = spawnSync('bash', ['-c', `exec 2>&1\n${script}`], { cwd: root, env, encoding: 'utf8', input: '', timeout: 300_000, maxBuffer: 64 * 1024 * 1024 });
    const parts = shown(root, result.stdout ?? '').split(new RegExp(`\\n${mark} \\d+\\n`));
    return cases.map((_, index) => trimmed((parts[index] ?? '').split('\n')));
  }

  /** The results of tool calls, each made with the server's root at `call.root`. */
  function callTools(env, calls) {
    const out = execFileSync(python, [join(scripts, 'mcp_examples.py')], { env, input: JSON.stringify(calls), encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'], maxBuffer: 64 * 1024 * 1024 });
    return JSON.parse(out);
  }

  /**
   * Runs the examples of one page. `tool` names the MCP tool that a page of the tool reference
   * is about. Returns how many examples ran.
   */
  function check(file, { tool } = {}) {
    const source = readFileSync(join(repository, file), 'utf8');
    const rewritten = source.split('\n');
    sessions += 1;
    const root = join(work, `session-${sessions}`);
    cpSync(template, root, { recursive: true });
    const env = sessionEnv(root);
    let count = 0;
    let pending = null;
    for (const block of blocks(source)) {
      const where = `${file}:${block.line}`;
      const title = metaValue(block.meta, 'title');
      if (title && !title.includes(' ')) {
        mkdirSync(dirname(join(root, title)), { recursive: true });
        writeFileSync(join(root, title), `${block.lines.join('\n')}\n`);
      } else if (block.language === 'console') {
        const cases = consoleCases(block);
        const real = runConsole(root, env, cases);
        if (write && !written.has(where) && cases.some((example, index) => !linesMatch(trimmed(example.shown), real[index]))) {
          written.add(where);
          rewritten.splice(block.line, block.lines.length, ...cases.flatMap((example, index) => [`$ ${example.command}`, ...real[index]]));
          writeFileSync(join(repository, file), rewritten.join('\n'));
          console.log(`examples.mjs: wrote what xpd prints into ${where}`);
          return check(file, { tool });
        }
        for (const [index, example] of cases.entries()) {
          count += 1;
          ran.commands.push(example.command);
          if (!linesMatch(trimmed(example.shown), real[index])) {
            problems.push(`${where}: $ ${example.command}\n  the page shows:\n${trimmed(example.shown).map((line) => `    ${line}`).join('\n')}\n  xpd prints:\n${real[index].map((line) => `    ${line}`).join('\n')}`);
          }
        }
      } else if (tool && block.language === 'json' && block.section === 'Example') {
        const text = block.lines.join('\n');
        if (!pending) {
          pending = { where, text, root: join(root, metaValue(block.meta, 'root') ?? '.') };
          continue;
        }
        count += 1;
        ran.tools.push(tool);
        const call = pending;
        pending = null;
        let parsed;
        try {
          parsed = { args: JSON.parse(call.text), result: JSON.parse(text) };
        } catch (error) {
          problems.push(`${call.where}: the arguments or the result of the example are not JSON: ${error.message}`);
          continue;
        }
        const [answer] = callTools(env, [{ root: call.root, tool, arguments: JSON.parse(call.text.replaceAll(SHOWN_ROOT, root)) }]);
        const real = answerValue(shown(root, answer.text), answer.failed);
        if (!valueMatches(parsed.result, real)) {
          problems.push(`${where}: ${tool} ${call.text.trim()}\n  the page shows:\n    ${text.trim()}\n  the tool returns:\n    ${JSON.stringify(real)}`);
        }
      }
    }
    if (pending) problems.push(`${pending.where}: the example shows the arguments of a call and no result`);
    return count;
  }

  /** Fails when an example differed from its page. */
  function finish() {
    if (problems.length) {
      throw new Error(`${problems.length} problems with the examples of the pages:\n\n${problems.join('\n\n')}`);
    }
    return ran;
  }

  return { check, finish, ran, report: (problem) => problems.push(problem) };
}
