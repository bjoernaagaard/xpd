// The rule packs in the curated order that the sidebar and the rules overview list them, and the
// single reader of rules/<pack>/pack.md shared by astro.config.mjs and scripts/rules.mjs.

import { readFileSync } from 'node:fs';
import { splitTitle } from './markdown.mjs';

export const PACKS = [
  'pandas',
  'numpy',
  'scipy',
  'sklearn',
  'sklego',
  'mapie',
  'category_encoders',
  'linearmodels',
  'arch',
  'pymoo',
  'plotly',
  'hamilton',
  'python',
  'xpd',
];

/** Orders packs discovered from `xpd rules` using `PACKS`, appending any new packs alphabetically. */
export function orderedPacks(packs) {
  const rank = new Map(PACKS.map((pack, index) => [pack, index]));
  return [...packs].sort((a, b) => {
    const rankA = rank.get(a) ?? Number.MAX_SAFE_INTEGER;
    const rankB = rank.get(b) ?? Number.MAX_SAFE_INTEGER;
    return rankA !== rankB ? rankA - rankB : a.localeCompare(b);
  });
}

/** A pack page's title and its Markdown below the leading `# <title>` heading. */
export function packPage(pack) {
  const text = readFileSync(new URL(`../../rules/${pack}/pack.md`, import.meta.url), 'utf8');
  return splitTitle(text, `rules/${pack}/pack.md`);
}
