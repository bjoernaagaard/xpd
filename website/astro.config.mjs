import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import mermaid from 'astro-mermaid';
import { readFileSync } from 'node:fs';
import { PACKS } from './src/packs.mjs';

/** The name of a rule pack, from the first line of rules/<pack>/pack.md, such as scikit-learn. */
const packTitle = (pack) =>
  readFileSync(new URL(`../rules/${pack}/pack.md`, import.meta.url), 'utf8')
    .split('\n')[0]
    .replace(/^# /, '');

// GitHub Pages serves the project site at https://bjoernaagaard.github.io/xpd/.
export default defineConfig({
  site: 'https://bjoernaagaard.github.io',
  base: '/xpd',
  output: 'static',
  // `mise run docs` builds the site into site/ at the root of the repository, which the Pages
  // workflow uploads.
  outDir: '../site',
  // Keep straight quotes, dashes and dots as written, so text copied from a page matches what a
  // reader types.
  markdown: { processor: satteri({ features: { smartPunctuation: false } }) },
  integrations: [
    // astro-mermaid has to come before Starlight, so it sees the mermaid code blocks first.
    mermaid({ autoTheme: true, enableLog: false }),
    starlight({
      title: 'xpd',
      description:
        'A fast linter for Python library habits, starting with pandas, NumPy, SciPy and scikit-learn.',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/bjoernaagaard/xpd' }],
      editLink: { baseUrl: 'https://github.com/bjoernaagaard/xpd/edit/main/website/' },
      sidebar: [
        { label: 'Get started', link: '/' },
        {
          label: 'Guides',
          items: [
            'guides/claude-code',
            'guides/mcp',
            'guides/code-mode',
            'guides/editors',
            'guides/write-a-rule',
          ],
        },
        {
          label: 'Rules',
          items: [
            { label: 'All rules', link: '/rules/' },
            ...PACKS.map((pack) => ({ label: packTitle(pack), link: `/rules/${pack}/` })),
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
