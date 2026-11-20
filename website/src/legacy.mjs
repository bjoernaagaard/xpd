// The addresses of the mdBook site that this site replaced, and where each one leads now. Released
// versions of xpd, editors and bookmarks still link to them, such as rules.html#chained-assignment.
//
// GitHub Pages can't redirect, and a browser never sends the part after # to the server, so each
// old page is a small HTML page whose script reads the fragment and opens the new page. The
// fragments are the heading ids of the old chapters, and a fragment that isn't listed opens the
// chapter's new page.

import { getCollection } from 'astro:content';

const headingSlug = (title) => title.toLowerCase().replace(/\s+/g, '-');

/** The new page of each old fragment of `rules.html`, derived from the pages scripts/rules.mjs
 * wrote from `xpd rules`. */
async function rulesFragments() {
  const fragments = {};
  const packs = [];
  for (const { id, data } of await getCollection('docs', ({ id }) => id.startsWith('rules/'))) {
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
    'when-a-rule-has-a-fix': 'guides/write-a-rule/#when-a-rule-gets-a-fix',
    'index-a-new-pack': 'guides/write-a-rule/#index-the-library-of-a-new-pack',
  };
}

/** The new page of each old fragment of `commands.html`, with top-level commands derived from the
 * pages scripts/reference.mjs wrote from `xpd commands`. */
async function commandsFragments() {
  const commands = {};
  for (const { id, data } of await getCollection('docs', ({ id }) => id.startsWith('commands/'))) {
    const words = data.title.split(' ');
    if (words[0] === 'xpd' && words.length === 2) {
      commands[words[1]] = `${id}/`;
    }
  }
  const indexPage = commands.index;
  return {
    ...commands,
    fixes: 'commands/check/#fixes',
    'output-and-exit-codes': 'commands/check/#exit-status',
    'select-rules': 'commands/check/#--rule',
    'project-rules': 'commands/check/#project-rules',
    'suppress-an-issue': 'commands/check/#suppress-an-issue',
    'step-code': 'commands/graph/',
    'method-chains': 'commands/graph/',
    'hamilton-modules': 'commands/graph/',
    'output-and-exit-code': 'commands/graph/#exit-status',
    ranking: 'commands/find/#ranking',
    'index-file': `${indexPage}#index-file`,
    'public-names': indexPage,
    recipes: `${indexPage}#recipes`,
    'discovery-questions': 'commands/find/#discovery-questions',
  };
}

const claudeCodeFragments = {
  'what-the-hook-does': 'guides/claude-code/#lint-each-edit-with-the-hook',
  'agent-plugin': 'guides/claude-code/#agent-plugin',
  'agent-skill': 'guides/claude-code/#agent-skill',
  'mcp-server': 'guides/mcp/',
};

const mcpFragments = {
  install: 'guides/mcp/#add-the-server-to-a-client',
  'how-a-call-runs': 'guides/mcp/#lint-and-fix-through-the-tools',
  tools: 'mcp/#tools',
  'confirmation-of-unsafe-fixes': 'mcp/tools/fix/',
  'background-tasks': 'mcp/#background-tasks',
  resources: 'mcp/resources/',
  prompts: 'mcp/prompts/',
  settings: 'guides/mcp/#settings',
  http: 'guides/mcp/#serve-several-clients-over-http',
  'other-ways-to-run-it': 'guides/mcp/#call-the-server-from-a-terminal',
  'library-discovery': 'guides/mcp/#look-up-a-library-before-writing-code',
  'the-discovery-record': 'guides/code-mode/#discovery-before-execution',
  'the-discovery-gate': 'mcp/tools/discovery_gate/',
};

const codeModeFragments = {
  'the-run-in-order': 'guides/code-mode/#run-the-stages-in-one-call',
  'discovery-before-execution': 'guides/code-mode/#discovery-before-execution',
  'the-stages': 'guides/code-mode/#the-stages',
  'the-gates': 'guides/code-mode/#the-gates',
  'a-run-from-raw-rows-to-a-saved-model': 'guides/code-mode/#before-you-start',
  'the-prediction-script': 'guides/code-mode/#the-stages',
  'a-leaky-script-stops-at-check': 'guides/code-mode/#a-leak-stops-the-run-at-check',
  'limits-that-no-gate-checks': 'guides/code-mode/#what-the-gates-dont-check',
};

const lspFragments = {
  'what-the-editor-gets': 'guides/editors/#what-the-editor-shows',
  diagnostics: 'guides/editors/#what-the-editor-shows',
  'code-actions': 'guides/editors/#fixes-as-code-actions',
  hover: 'guides/editors/#rule-help-on-hover',
  'chained-assignment': 'guides/editors/#rule-help-on-hover',
  'the-project-and-its-venv': 'guides/editors/#the-project-its-rules-and-its-virtual-environment',
  'project-rules': 'guides/editors/#the-project-its-rules-and-its-virtual-environment',
  settings: 'guides/editors/#settings',
  'what-the-server-leaves-out': 'guides/editors/#what-the-server-leaves-out',
  'set-up-an-editor': 'guides/editors/#set-up-an-editor',
  neovim: 'guides/editors/#neovim',
  helix: 'guides/editors/#helix',
  zed: 'guides/editors/#zed',
  'vs-code-and-editors-built-on-it': 'guides/editors/#vs-code-and-editors-built-on-it',
  'another-editor': 'guides/editors/#another-editor',
};

/**
 * Each old page with the new page it opens and the new address of each old fragment. The
 * mdBook site also answered at <chapter>/index.html, and those addresses for commands, mcp and
 * rules are pages of this site, so only the other three need a page of their own.
 */
export async function legacyPages() {
  return [
    { path: 'commands.html', page: 'commands/', fragments: await commandsFragments() },
    { path: 'claude-code.html', page: 'guides/claude-code/', fragments: claudeCodeFragments },
    { path: 'claude-code/index.html', page: 'guides/claude-code/', fragments: claudeCodeFragments },
    { path: 'mcp.html', page: 'guides/mcp/', fragments: mcpFragments },
    { path: 'code-mode.html', page: 'guides/code-mode/', fragments: codeModeFragments },
    { path: 'code-mode/index.html', page: 'guides/code-mode/', fragments: codeModeFragments },
    { path: 'lsp.html', page: 'guides/editors/', fragments: lspFragments },
    { path: 'lsp/index.html', page: 'guides/editors/', fragments: lspFragments },
    { path: 'rules.html', page: 'rules/', fragments: await rulesFragments() },
  ];
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
