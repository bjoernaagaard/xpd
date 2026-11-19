// The old chapter pages, such as rules.html, which open their new pages. See src/legacy.mjs.
import { legacyHtml, legacyPages } from '../legacy.mjs';

export async function getStaticPaths() {
  return (await legacyPages())
    .filter(({ path }) => !path.includes('/'))
    .map((legacy) => ({ params: { name: legacy.path.replace(/\.html$/, '') }, props: legacy }));
}

export function GET({ props }) {
  return new Response(legacyHtml(import.meta.env.SITE, import.meta.env.BASE_URL, props), {
    headers: { 'Content-Type': 'text/html; charset=utf-8' },
  });
}
