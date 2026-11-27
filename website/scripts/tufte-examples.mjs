import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { editUrl, page, yaml } from '../src/markdown.mjs';
import { run } from './exec.mjs';
import { proseIssues } from './prose-lint.mjs';
import { validateReview } from './tufte-review.mjs';

const script = fileURLToPath(import.meta.url);
const website = resolve(dirname(script), '..');
const repository = dirname(website);
const sources = join(website, 'examples', 'tufte');
const astro = join(website, '.astro');
const pythonHelper = join(website, 'scripts', 'tufte-example.py');
const variants = ['before', 'after'];
const gallery = 'guides/tufte-examples';
const shellQuote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
const step = (command, args) => run(command, args, { cwd: repository, timeout: 60_000 });
const text = (value) => typeof value === 'string' && value.trim().length > 0;

function readCatalog(xpd, ids) {
  const { questions } = JSON.parse(step(xpd, ['chart-review', '--questions', '--format', 'json']));
  const files = readdirSync(sources, { withFileTypes: true });
  const names = new Set(files.filter((file) => file.isFile()).map(({ name }) => name));
  const known = new Set(questions.map(({ id }) => id));
  const duplicates = questions.filter(({ id }, index) => questions.findIndex((question) => question.id === id) !== index).map(({ id }) => id);
  const missing = [...known].filter((id) => !names.has(`${id}.py`));
  const extra = files.filter(({ name }) => !names.has(name) || !name.endsWith('.py') || !known.has(name.slice(0, -3))).map(({ name }) => name);
  console.log(`Tufte coverage\nmissing: ${JSON.stringify(missing)}\nextra: ${JSON.stringify(extra)}\nduplicates: ${JSON.stringify(duplicates)}`);
  if (extra.length || duplicates.length || (ids ? ids.some((id) => !known.has(id) || missing.includes(id)) : missing.length)) {
    throw new Error('Tufte example coverage failed');
  }
  return ids ? questions.filter(({ id }) => ids.includes(id)) : questions;
}

function readMetadata(python, sections, work, assets, source) {
  const preamble = join(work, 'preamble.py');
  const metadata = join(work, 'metadata.py');
  writeFileSync(preamble, sections[0]);
  writeFileSync(metadata, sections[3]);
  const result = JSON.parse(step(python, [pythonHelper, 'metadata', preamble, metadata, assets]));
  const { example } = result;
  const strings = [example?.source, example?.question, example?.book?.quote,
    ...variants.flatMap((variant) => [example?.[variant]?.caption, example?.[variant]?.summary])];
  if (!strings.every(text) || !['page', 'line'].every((key) => Number.isSafeInteger(example.book[key]) && example.book[key] > 0)) {
    throw new Error('EXAMPLE needs source, question, book quote, page and line, and both captions and summaries');
  }
  const issues = proseIssues(`Source. ${strings.join('\n\n')}`, source);
  if (issues.length) throw new Error(issues.join('\n'));
  return result;
}

function reviewVariant({ xpd, python, variant, sections, work }) {
  const file = join(work, `${variant}.py`);
  const figure = join(work, `${variant}.json`);
  const payload = join(work, `${variant}.adapter.json`);
  const section = sections[variants.indexOf(variant) + 1];
  writeFileSync(file, `${sections[0].trimEnd()}\n\n\n${section.trim()}\n\n\nif __name__ == "__main__":\n    print(make_${variant}(load_data()).to_json())\n`);
  writeFileSync(figure, step(python, [file]));
  writeFileSync(payload, step(python, [pythonHelper, 'adapter', figure, join(repository, 'py', 'figure_facts.py'), variant]));
  const report = JSON.parse(step(xpd, ['chart-review', file, '--adapter', `cat ${shellQuote(payload)}`, '--format', 'json', '--limit', '0']));
  const runtime = report.read?.facts;
  const measured = runtime?.figures?.[0];
  if (report.read?.files !== 1 || report.read?.python !== 1 || runtime?.error || runtime?.figures?.length !== 1
    || measured.name !== variant || measured.library !== 'plotly' || measured.unresolved !== null
    || !measured.facts || !Array.isArray(report.findings)) {
    throw new Error(`expected one scanned Python file and one measured Plotly figure: ${JSON.stringify(report)}`);
  }
  return report;
}

function generateExample(question, paths, xpd, python, version, order) {
  const { id } = question;
  const source = join(sources, `${id}.py`);
  try {
    const sections = JSON.parse(step(python, [pythonHelper, 'split', source]));
    const work = join(paths.work, id);
    const assets = join(paths.assets, id);
    mkdirSync(work);
    mkdirSync(assets);
    const { example, table } = readMetadata(python, sections, work, assets, source);
    const reports = Object.fromEntries(variants.map((variant) => {
      try {
        return [variant, reviewVariant({ xpd, python, variant, sections, work })];
      } catch (error) {
        throw new Error(`${variant}: ${error.message}`, { cause: error });
      }
    }));
    const publicVariants = validateReview(question, reports);
    for (const [index, variant] of variants.entries()) {
      copyFileSync(join(work, `${variant}.py`), join(assets, `${variant}.py`));
      if (publicVariants[index]) copyFileSync(join(work, `${variant}.json`), join(assets, `${variant}.json`));
    }
    const props = { id, example, table, version, publicVariants, csv: existsSync(join(assets, 'values.csv')), rules: question.rules };
    writeFileSync(join(paths.pages, `${id}.mdx`), page({
      title: yaml(question.principle),
      description: yaml(example.after.caption),
      editUrl: yaml(editUrl(`website/examples/tufte/${id}.py`)),
      sidebar: { order },
    }, `import TufteExample from '../../../../components/TufteExample.astro';\n\n<TufteExample {...${JSON.stringify(props)}} />`));
  } catch (error) {
    throw new Error(`${id} (${source}): ${error.message}`, { cause: error });
  }
}

export function generateTufteExamples(xpd, python, ids) {
  const questions = readCatalog(xpd, ids).sort((a, b) => a.line - b.line);
  const exampleIds = new Set(questions.map(({ id }) => id));
  mkdirSync(astro, { recursive: true });
  const staging = mkdtempSync(join(astro, 'tufte-stage-'));
  const paths = { pages: join(staging, 'pages'), assets: join(staging, 'assets'), work: join(staging, 'work') };

  try {
    Object.values(paths).forEach((folder) => mkdirSync(folder));
    const version = step(python, [pythonHelper, 'bundle', paths.assets]).trim();
    console.log('Principle | Before findings | Before failed facts | After findings | After failed facts');
    for (const [index, question] of questions.entries()) generateExample(question, paths, xpd, python, version, index + 1);
    const index = questions.map(({ id, principle }) => `- [${principle.replaceAll('[', '\\[').replaceAll(']', '\\]')}](./${id}/)`).join('\n');
    writeFileSync(join(paths.pages, 'index.md'), page({ title: yaml('Tufte examples'), editUrl: false, sidebar: { order: 0 } },
      `Compare two Plotly displays of the same data for each principle. The examples follow the order of Tufte's book.\n\n${index}`));
    const targets = [
      [paths.pages, join(website, 'src', 'content', 'docs', gallery)],
      [paths.assets, join(website, 'public', 'tufte-examples')],
      [paths.work, join(astro, 'tufte-examples')],
    ];
    for (const [folder, target] of targets) {
      rmSync(target, { recursive: true, force: true });
      renameSync(folder, target);
    }
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
  return exampleIds;
}

if (process.argv[1] && resolve(process.argv[1]) === script) {
  const { values } = parseArgs({ options: { id: { type: 'string', multiple: true }, xpd: { type: 'string' }, python: { type: 'string' } } });
  if (!values.xpd || !values.python) throw new Error('usage: tufte-examples.mjs [--id <id>] --xpd <binary> --python <interpreter>');
  generateTufteExamples(resolve(values.xpd), resolve(values.python), values.id);
}
