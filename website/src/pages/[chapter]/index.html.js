// The old <chapter>/index.html addresses that no page of this site has. See src/legacy.mjs.
import { legacyHtml, legacyPages } from '../../legacy.mjs';

export async function getStaticPaths() {
  return (await legacyPages())
    .filter(({ path }) => path.endsWith('/index.html'))
    .map((legacy) => ({ params: { chapter: legacy.path.split('/')[0] }, props: legacy }));
}

export function GET({ props }) {
  return new Response(legacyHtml(import.meta.env.SITE, import.meta.env.BASE_URL, props), {
    headers: { 'Content-Type': 'text/html; charset=utf-8' },
  });
}
