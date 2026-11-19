import { defineEcConfig } from '@astrojs/starlight/expressive-code';
import { pluginLineNumbers } from '@expressive-code/plugin-line-numbers';

// Long lines wrap, so a reader on a phone can read a whole line without scrolling sideways. Line
// numbers are off by default, and a code block turns them on with showLineNumbers.
export default defineEcConfig({
  plugins: [pluginLineNumbers()],
  defaultProps: { wrap: true, showLineNumbers: false },
});
