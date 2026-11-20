// Builds xpd once and generates all rule, command and MCP documentation pages from that binary.

import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { buildXpd } from './exec.mjs';
import { generateReference } from './reference.mjs';
import { generateRules } from './rules.mjs';

const repository = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const xpd = buildXpd(repository);
generateRules(xpd);
await generateReference(xpd);
