// The addresses of the mdBook site that this site replaced, and where each one leads now. Released
// versions of xpd, editors and bookmarks still link to them, such as rules.html#chained-assignment.
//
// GitHub Pages can't redirect, and a browser never sends the part after # to the server, so each
// old page is a small HTML page whose script reads the fragment and opens the new page. The
// fragments are the heading ids of the old chapters, and a fragment that isn't listed opens the
// chapter's new page.

import { getCollection, render } from 'astro:content';

const headingSlug = (title) => title.toLowerCase().replace(/\s+/g, '-');

/** The new page of each old fragment of `rules.html`, derived from the pages scripts/rules.mjs
 * wrote from `xpd rules`. */
function rulesFragments(docs) {
  const fragments = {};
  const packs = [];
  for (const { id, data } of docs) {
    if (!id.startsWith('rules/')) continue;
    const [, pack, rule] = id.split('/');
    if (rule) fragments[rule] = `rules/${pack}/${rule}/`;
    else if (pack) packs.push([headingSlug(data.title), pack]);
  }
  for (const [heading, pack] of packs) {
    fragments[heading] ??= `rules/${pack}/`;
  }
  return {
    ...fragments,
    'write-a-rule': 'guides/write-a-rule/',
    'when-a-rule-has-a-fix': 'guides/write-a-rule/#contribute-a-rule-to-xpd',
    'index-a-new-pack': 'guides/write-a-rule/#index-a-new-library-pack',
  };
}

/** The new page of each old fragment of `commands.html`, with top-level commands derived from the
 * pages scripts/reference.mjs wrote from `xpd commands`. */
function commandsFragments(docs) {
  const commands = {};
  for (const { id, data } of docs) {
    if (!id.startsWith('commands/')) continue;
    const words = data.title.split(' ');
    if (words[0] === 'xpd' && words.length === 2) {
      commands[words[1]] = `${id}/`;
    }
  }
  const indexPage = commands.index;
  return {
    ...commands,
    fixes: 'commands/check/#fix-modes',
    'output-and-exit-codes': 'commands/check/#exit-status',
    'select-rules': 'commands/check/#options',
    'project-rules': 'commands/check/#project-rules',
    'suppress-an-issue': 'commands/check/#suppress-an-issue',
    'step-code': 'commands/graph/',
    'method-chains': 'commands/graph/',
    'hamilton-modules': 'commands/graph/',
    'output-and-exit-code': 'commands/graph/#exit-status',
    ranking: 'commands/find/#hybrid-ranking-and-offline-cache',
    'index-file': `${indexPage}#index-file`,
    'public-names': indexPage,
    recipes: `${indexPage}#recipes`,
    'discovery-questions': 'commands/find/#playbooks',
  };
}

const claudeCodeFragments = {
  'what-the-hook-does': 'guides/claude-code/#lint-each-edit-with-the-hook',
  'agent-plugin': 'guides/claude-code/#agent-plugin',
  'agent-skill': 'guides/claude-code/#agent-skill',
  'mcp-server': 'guides/mcp/',
};

const mcpFragments = {
  install: 'guides/mcp/#connect-a-client',
  'how-a-call-runs': 'mcp/#how-a-call-runs',
  tools: 'mcp/#tools',
  'confirmation-of-unsafe-fixes': 'commands/check/',
  'background-tasks': 'mcp/#errors-and-background-tasks',
  resources: 'mcp/resources/',
  prompts: 'mcp/prompts/',
  settings: 'guides/mcp/#project-root-and-security',
  http: 'guides/mcp/#serve-over-http-or-test-from-a-shell',
  'other-ways-to-run-it': 'guides/mcp/#serve-over-http-or-test-from-a-shell',
  'library-discovery': 'commands/find/',
  'the-discovery-record': 'guides/gates/#pipeline-contract-stages',
  'the-discovery-gate': 'mcp/tools/discovery_gate/',
};

const codeModeFragments = {
  'the-run-in-order': 'guides/gates/#run-the-sample-pipeline',
  'discovery-before-execution': 'guides/gates/#pipeline-contract-stages',
  'the-stages': 'guides/gates/#pipeline-contract-stages',
  'the-gates': 'guides/gates/#pipeline-contract-stages',
  'a-run-from-raw-rows-to-a-saved-model': 'guides/gates/#run-the-sample-pipeline',
  'the-prediction-script': 'guides/gates/#pipeline-contract-stages',
  'a-leaky-script-stops-at-check': 'guides/gates/#run-the-sample-pipeline',
  'limits-that-no-gate-checks': 'guides/gates/#known-limits',
};

const lspFragments = {
  'what-the-editor-gets': 'guides/editors/#code-actions-hover-and-cell-files',
  diagnostics: 'guides/editors/#code-actions-hover-and-cell-files',
  'code-actions': 'guides/editors/#code-actions-hover-and-cell-files',
  hover: 'guides/editors/#code-actions-hover-and-cell-files',
  'chained-assignment': 'guides/editors/#code-actions-hover-and-cell-files',
  'the-project-and-its-venv': 'guides/editors/#workspace-resolution-and-scope',
  'project-rules': 'guides/editors/#workspace-resolution-and-scope',
  settings: 'guides/editors/#workspace-resolution-and-scope',
  'what-the-server-leaves-out': 'guides/editors/#workspace-resolution-and-scope',
  'set-up-an-editor': 'guides/editors/#set-up-an-editor',
  neovim: 'guides/editors/#neovim',
  helix: 'guides/editors/#helix',
  zed: 'guides/editors/#zed',
  'vs-code-and-editors-built-on-it': 'guides/editors/#vs-code-and-editors-built-on-it',
  'another-editor': 'guides/editors/#another-editor',
};

/** Verifies that every page and `#heading` target in `pages` exists in the generated docs. */
async function verifyTargets(docs, pages) {
  const byPath = new Map(docs.map((entry) => [entry.id ? `${entry.id}/` : '', entry]));
  const headingSets = new Map();
  async function headingsOf(pagePath) {
    if (!headingSets.has(pagePath)) {
      const entry = byPath.get(pagePath);
      const headings = entry.rendered?.metadata?.headings ?? (await render(entry)).headings;
      headingSets.set(pagePath, new Set(headings.map((heading) => heading.slug)));
    }
    return headingSets.get(pagePath);
  }

  const problems = [];
  for (const { path, page, fragments } of pages) {
    if (!byPath.has(page)) {
      problems.push(`${path} -> ${page}: target page does not exist`);
    }
    for (const [fragment, target] of Object.entries(fragments)) {
      const hash = target.indexOf('#');
      const targetPage = hash === -1 ? target : target.slice(0, hash);
      const anchor = hash === -1 ? '' : target.slice(hash + 1);
      if (!byPath.has(targetPage)) {
        problems.push(`${path}#${fragment} -> ${target}: target page "${targetPage}" does not exist`);
        continue;
      }
      if (anchor && !(await headingsOf(targetPage)).has(anchor)) {
        problems.push(`${path}#${fragment} -> ${target}: heading "#${anchor}" does not exist on "${targetPage}"`);
      }
    }
  }
  if (problems.length) {
    throw new Error(`legacy.mjs points at missing pages or headings:\n${problems.join('\n')}`);
  }
}

let cachedPages;

/**
 * Each old page with the new page it opens and the new address of each old fragment. The
 * mdBook site also answered at <chapter>/index.html, and those addresses for commands, mcp and
 * rules are pages of this site, so only the other three need a page of their own.
 */
export function legacyPages() {
  cachedPages ??= (async () => {
    const docs = await getCollection('docs');
    const pages = [
      { path: 'commands.html', page: 'commands/', fragments: commandsFragments(docs) },
      { path: 'claude-code.html', page: 'guides/claude-code/', fragments: claudeCodeFragments },
      { path: 'claude-code/index.html', page: 'guides/claude-code/', fragments: claudeCodeFragments },
      { path: 'mcp.html', page: 'guides/mcp/', fragments: mcpFragments },
      { path: 'code-mode.html', page: 'guides/gates/', fragments: codeModeFragments },
      { path: 'code-mode/index.html', page: 'guides/gates/', fragments: codeModeFragments },
      { path: 'lsp.html', page: 'guides/editors/', fragments: lspFragments },
      { path: 'lsp/index.html', page: 'guides/editors/', fragments: lspFragments },
      { path: 'rules.html', page: 'rules/', fragments: rulesFragments(docs) },
    ];
    await verifyTargets(docs, pages);
    return pages;
  })();
  return cachedPages;
}

/** The HTML of an old page that opens `page`, or the new address of the fragment it was opened
 * with. Without JavaScript, the browser opens `page` after the refresh. */
export function legacyHtml(site, base, { page, fragments }) {
  const root = base.endsWith('/') ? base : `${base}/`;
  const target = `${root}${page}`;
  const canonical = new URL(target, site).href;
  const script = `const to=${JSON.stringify(fragments)};const id=decodeURIComponent(location.hash.slice(1));location.replace(${JSON.stringify(root)}+(Object.hasOwn(to,id)?to[id]:${JSON.stringify(page)}));`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>xpd: this page has moved</title>
<meta name="robots" content="noindex">
<link rel="canonical" href="${canonical}">
<script>${script}</script>
<noscript><meta http-equiv="refresh" content="0; url=${target}"></noscript>
</head>
<body>
<p>This page has moved to <a href="${target}">${target}</a>.</p>
</body>
</html>
`;
}

/** Builds Astro `getStaticPaths` and `GET` handlers for a subset of legacy redirect pages. */
export function legacyEndpoint(match, paramsOf) {
  return {
    async getStaticPaths() {
      return (await legacyPages())
        .filter(({ path }) => match(path))
        .map((legacy) => ({ params: paramsOf(legacy.path), props: legacy }));
    },
    GET({ props }) {
      return new Response(legacyHtml(import.meta.env.SITE, import.meta.env.BASE_URL, props), {
        headers: { 'Content-Type': 'text/html; charset=utf-8' },
      });
    },
  };
}
