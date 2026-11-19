import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import mermaid from 'astro-mermaid';

// GitHub Pages serves the project site at https://bjoernaagaard.github.io/xpd/.
export default defineConfig({
  site: 'https://bjoernaagaard.github.io',
  base: '/xpd',
  output: 'static',
  // Keep straight quotes, dashes and dots as written, as the mdBook site did, so text copied from a
  // page matches what a reader types.
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
      sidebar: [{ label: 'Get started', link: '/' }],
      pagefind: true,
    }),
  ],
});
