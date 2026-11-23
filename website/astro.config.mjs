import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import mermaid from 'astro-mermaid';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { editUrl, REPO_URL } from './src/markdown.mjs';
import { PACKS, packPage } from './src/packs.mjs';

function rulePackSidebar() {
  const generated = join(dirname(fileURLToPath(import.meta.url)), '.astro', 'rule-packs.json');
  if (existsSync(generated)) {
    return JSON.parse(readFileSync(generated, 'utf8'));
  }
  return PACKS.map((pack) => ({ label: packPage(pack).title, link: `/rules/${pack}/` }));
}

const CODE_MODE_SCRIPT_ID = 'virtual:xpd-code-mode-script';
const RESOLVED_CODE_MODE_SCRIPT_ID = `\0${CODE_MODE_SCRIPT_ID}`;

/** Supplies the `build_pipeline` code-mode script from `python/xpd/mcp/prompts.py` at build time. */
function codeModeScriptPlugin() {
  return {
    name: 'xpd-code-mode-script',
    resolveId(id) {
      if (id === CODE_MODE_SCRIPT_ID) return RESOLVED_CODE_MODE_SCRIPT_ID;
    },
    load(id) {
      if (id !== RESOLVED_CODE_MODE_SCRIPT_ID) return;
      const website = dirname(fileURLToPath(import.meta.url));
      const repository = join(website, '..');
      const venvPython = join(
        website,
        '.mcp-venv',
        process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python',
      );
      const python = existsSync(venvPython) ? venvPython : 'python3';
      const script = execFileSync(
        python,
        [
          '-c',
          'import json; from xpd.mcp.prompts import stages_script; ' +
            'print(json.dumps(stages_script("penguins.csv", "species", "classify penguin species with a logistic regression pipeline")))',
        ],
        {
          cwd: repository,
          encoding: 'utf8',
          env: { ...process.env, PYTHONPATH: join(repository, 'python') },
        },
      ).trim();
      return `export default ${script};\n`;
    },
  };
}

// GitHub Pages serves the project site at https://bjoernaagaard.github.io/xpd/.
export default defineConfig({
  site: 'https://bjoernaagaard.github.io',
  base: '/xpd',
  output: 'static',
  // `mise run docs` builds the site into site/ at the root of the repository, which the Pages
  // workflow uploads.
  outDir: '../site',
  vite: { plugins: [codeModeScriptPlugin()] },
  // Keep straight quotes, dashes and dots as written, so text copied from a page matches what a
  // reader types.
  markdown: { processor: satteri({ features: { smartPunctuation: false } }) },
  integrations: [
    // astro-mermaid has to come before Starlight, so it sees the mermaid code blocks first.
    mermaid({ autoTheme: true, enableLog: false }),
    starlight({
      title: 'xpd',
      description: 'A fast linter for Python library habits.',
      social: [{ icon: 'github', label: 'GitHub', href: REPO_URL }],
      editLink: { baseUrl: editUrl('website/') },
      sidebar: [
        { label: 'Get started', link: '/' },
        { label: 'Guides', items: [{ autogenerate: { directory: 'guides' } }] },
        {
          label: 'Rules',
          items: [
            { label: 'All rules', link: '/rules/' },
            ...rulePackSidebar(),
          ],
        },
        // scripts/reference.mjs writes these pages from the CLI and the MCP server.
        { label: 'Commands', collapsed: true, items: [{ autogenerate: { directory: 'commands' } }] },
        { label: 'MCP server', collapsed: true, items: [{ autogenerate: { directory: 'mcp' } }] },
      ],
      pagefind: true,
    }),
  ],
});
