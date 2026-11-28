import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { editUrl, REPO_URL } from './src/markdown.mjs';
import { PACKS, packPage } from './src/packs.mjs';
import { peleMermaidPlugin } from './src/pele.mjs';

function rulePackSidebar() {
  const generated = join(dirname(fileURLToPath(import.meta.url)), '.astro', 'rule-packs.json');
  if (existsSync(generated)) {
    return JSON.parse(readFileSync(generated, 'utf8'));
  }
  return PACKS.map((pack) => ({ label: packPage(pack).title, link: `/rules/${pack}/` }));
}

// GitHub Pages serves the project site at https://bjoernaagaard.github.io/xpd/.
export default defineConfig({
  site: 'https://bjoernaagaard.github.io',
  base: '/xpd',
  output: 'static',
  // `mise run docs` builds the site into site/ at the root of the repository, which the Pages
  // workflow uploads.
  outDir: '../site',
  markdown: {
    processor: satteri({
      features: { smartPunctuation: false },
      mdastPlugins: [peleMermaidPlugin()],
    }),
  },
  integrations: [
    starlight({
      title: 'xpd',
      description: 'Static linter, library index, and recorded review workbench for Python.',
      customCss: ['./src/styles/pele.css'],
      social: [{ icon: 'github', label: 'GitHub', href: REPO_URL }],
      editLink: { baseUrl: editUrl('website/') },
      sidebar: [
        { label: 'Get started', link: '/' },
        {
          label: 'Core guides',
          items: [
            ...['audit', 'libraries', 'claude-code', 'mcp', 'editors', 'write-a-rule'].map((id) => ({
              slug: `guides/${id}`,
            })),
          ],
        },
        {
          label: 'Code quality ledgers',
          collapsed: true,
          items: [
            ...['untangle', 'optimize', 'harden', 'discover'].map((id) => ({ slug: `guides/${id}` })),
          ],
        },
        {
          label: 'Data and ML ledgers',
          collapsed: true,
          items: [
            ...['data-prep', 'ts-review', 'feature-review', 'model-review', 'opt-review', 'chart-review', 'gates'].map(
              (id) => ({ slug: `guides/${id}` }),
            ),
          ],
        },
        {
          label: 'Media and gallery',
          collapsed: true,
          items: [
            { slug: 'guides/animate' },
            {
              label: 'Tufte gallery',
              collapsed: true,
              items: [{ autogenerate: { directory: 'guides/tufte-examples' } }],
            },
          ],
        },
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
