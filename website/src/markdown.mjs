// Shared Markdown scanning, frontmatter formatting and repository URL helpers used by the site
// configuration and build scripts.

export const REPO_URL = 'https://github.com/bjoernaagaard/xpd';

export const editUrl = (path) => `${REPO_URL}/edit/main/${path}`;

/** The text of a YAML scalar, as JSON, which YAML reads as the same string. */
export const yaml = (value) => JSON.stringify(value);

/** Renders a Markdown page with YAML frontmatter (scalars and one-level nested maps). */
export function page(frontmatter, body) {
  const lines = Object.entries(frontmatter).map(([key, value]) =>
    typeof value === 'object'
      ? `${key}:\n${Object.entries(value).map(([k, v]) => `  ${k}: ${v}`).join('\n')}`
      : `${key}: ${value}`,
  );
  return `---\n${lines.join('\n')}\n---\n\n${body.trim()}\n`;
}

/** Splits a document that starts with `# <title>` into its title and the Markdown below it. */
export function splitTitle(source, label) {
  const [first, ...rest] = source.split('\n');
  if (!first.startsWith('# ')) {
    throw new Error(`${label} must start with "# <title>"`);
  }
  return { title: first.slice(2).trim(), body: rest.join('\n') };
}

/** The first sentence of the first paragraph of plain or Markdown text, with line breaks joined. */
export function firstSentence(value) {
  const paragraph = String(value ?? '')
    .trim()
    .split(/\n\s*\n/)[0]
    .split('\n')
    .map((line) => line.trim())
    .join(' ');
  return paragraph.match(/^.*?[.!?](?=\s|$)/)?.[0] ?? paragraph;
}

/**
 * Scans Markdown once for fenced code blocks and `## ` sections.
 * Returns per-line fence state, extracted fenced blocks, the intro before the first `## ` heading,
 * and each `## ` section with its heading and trimmed body.
 */
export function parseMarkdown(source) {
  const lines = [];
  const blocks = [];
  const intro = [];
  const sections = [];
  let open = null;
  let current = null;

  for (const [index, line] of source.split('\n').entries()) {
    const section = current?.heading ?? '';
    if (open) {
      const closing = new RegExp(`^ {0,3}${open.fence[0]}{${open.fence.length},}\\s*$`).test(line);
      if (closing) {
        blocks.push(open);
        open = null;
        lines.push({ line, number: index + 1, fenced: false, isFence: true, section });
      } else {
        open.lines.push(line);
        lines.push({ line, number: index + 1, fenced: true, isFence: false, section });
      }
      (current ? current.lines : intro).push(line);
      continue;
    }

    const start = line.match(/^ {0,3}(`{3,}|~{3,})\s*([\w-]*)\s*(.*)$/);
    if (start) {
      open = { fence: start[1], language: start[2], meta: start[3], lines: [], line: index + 1, section };
      lines.push({ line, number: index + 1, fenced: false, isFence: true, section });
      (current ? current.lines : intro).push(line);
    } else if (line.startsWith('## ')) {
      current = { heading: line.slice(3).trim(), lines: [] };
      sections.push(current);
      lines.push({ line, number: index + 1, fenced: false, isFence: false, section: current.heading });
    } else {
      lines.push({ line, number: index + 1, fenced: false, isFence: false, section });
      (current ? current.lines : intro).push(line);
    }
  }

  return {
    lines,
    blocks,
    intro: intro.join('\n').trim(),
    sections: sections.map(({ heading, lines: body }) => ({ heading, body: body.join('\n').trim() })),
  };
}

export const sectionsText = (sections) =>
  sections.map((section) => `## ${section.heading}\n\n${section.body}`).join('\n\n');
