// The old chapter pages, such as rules.html, which open their new pages. See src/legacy.mjs.
import { legacyEndpoint } from '../legacy.mjs';

export const { getStaticPaths, GET } = legacyEndpoint(
  (path) => !path.includes('/'),
  (path) => ({ name: path.replace(/\.html$/, '') }),
);
