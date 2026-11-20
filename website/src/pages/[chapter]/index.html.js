// The old <chapter>/index.html addresses that no page of this site has. See src/legacy.mjs.
import { legacyEndpoint } from '../../legacy.mjs';

export const { getStaticPaths, GET } = legacyEndpoint(
  (path) => path.endsWith('/index.html'),
  (path) => ({ chapter: path.split('/')[0] }),
);
