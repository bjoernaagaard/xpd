import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import mermaid from 'astro-mermaid';
import { editUrl, REPO_URL } from './src/markdown.mjs';
import { PACKS, packPage } from './src/packs.mjs';

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
            ...PACKS.map((pack) => ({ label: packPage(pack).title, link: `/rules/${pack}/` })),
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
