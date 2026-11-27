import { createHash } from 'node:crypto';
import { PeleError, render } from 'pele';

export function peleMermaidPlugin() {
  const seenPerFile = new Map();

  return {
    name: 'pele-mermaid',
    code(node, context) {
      if (node.lang !== 'mermaid') return;

      const file = context?.fileURL?.pathname || 'unknown';
      const index = seenPerFile.get(file) ?? 0;
      seenPerFile.set(file, index + 1);

      const digest = createHash('sha256')
        .update(file)
        .update('\0')
        .update(String(index))
        .update('\0')
        .update(node.value)
        .digest('hex')
        .slice(0, 8);
      const idPrefix = `p${digest}`;

      try {
        const { svg } = render(node.value, { idPrefix });
        return {
          type: 'html',
          value: `<div class="pele-diagram not-content">${svg}</div>`,
        };
      } catch (error) {
        if (error instanceof PeleError) {
          const location = error.line > 0 ? `:${error.line}:${error.column}` : '';
          throw new Error(`pele failed on mermaid block in ${file}${location}: ${error.message}`, {
            cause: error,
          });
        }
        throw error;
      }
    },
  };
}
