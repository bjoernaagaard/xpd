const runtimeKinds = {
  compare: ['free-facet-scales'],
  context: ['missing-context'],
  'color-deficient': ['red-green-only'],
  'deflate-money': ['undeflated-money'],
  'horizontal-shape': ['vertical-aspect-ratio', 'poor-banking'],
  'labels-on-graphic': ['legend-instead-of-labels'],
  lines: ['thick-line'],
  'lines-of-sight': ['free-facet-scales'],
  'meaningful-id-order': ['alphabetical-order'],
  order: ['table-alphabetical-order'],
  puzzle: ['legend-instead-of-labels'],
  'scale-choice': ['poor-banking'],
  'table-alignment-and-precision': ['table-number-alignment', 'table-unformatted-numbers'],
  'table-header-clarity': ['table-raw-header'],
  'table-rules-and-borders': ['table-vertical-rules', 'table-zebra-striping'],
  'table-topical-groups': ['table-ungrouped-rows'],
  'zero-and-missing': ['zero-missing-ambiguity'],
};

function required(facts, path, accepts) {
  const value = path.split('.').reduce((parent, key) => parent?.[key], facts);
  if (!accepts(value)) throw new Error(`missing required measurement ${path}`);
  return value;
}

const positive = (value) => Number.isFinite(value) && value > 0;
const strings = (value) => Array.isArray(value) && value.every((item) => typeof item === 'string');
const string = (value) => typeof value === 'string';
const fuelEconomyUnit = /\b(?:miles per gallon|mpg)\b/i;
const deadline = /\b(?:19|20)\d\d\b/;

const predicates = {
  'horizontal-shape': (_, facts) => [
    ['wide canvas', required(facts, 'canvas.width', positive) > required(facts, 'canvas.height', positive)],
  ],
  labels: (_, facts) => [
    ['fuel economy unit', required(facts, 'text.axis_titles', strings).some((title) => fuelEconomyUnit.test(title))],
    ['deadline annotations', required(facts, 'text.annotations', strings).filter((note) => deadline.test(note)).length >= 2],
  ],
  'data-positioned-labels': (_, facts) => {
    const annotations = required(facts, 'text.annotations', strings);
    return [
      ['numeric annotations', annotations.filter((note) => /\d/.test(note)).length >= 2],
      ['fractional annotation', annotations.some((note) => /\d(?:[.,]\d|\s*\/\s*\d|\s*½)/.test(note))],
    ];
  },
  'table-caption-and-source': (_, facts) => [
    ['table title', required(facts, 'text.title', string).trim().length > 0],
    ['percent units', required(facts, 'table-structure.headers', strings).some((header) => /%|percent/i.test(header))],
  ],
  'many-numbers': (variant, facts, beforeFacts) => {
    const beforeDensity = required(beforeFacts, 'values.per_square_inch', Number.isFinite);
    const density = required(facts, 'values.per_square_inch', Number.isFinite);
    return [['greater value density', variant === 'after' && density > beforeDensity]];
  },
};

export function validateReview(question, variants) {
  const kinds = new Set([...question.rule_finding_kinds, ...(runtimeKinds[question.id] ?? [])]);
  const predicate = predicates[question.id];
  const results = Object.entries(variants).map(([variant, report]) => {
    const facts = report.read.facts.figures[0].facts;
    const failed = (predicate?.(variant, facts, variants.before.read.facts.figures[0].facts) ?? [])
      .filter(([, passed]) => !passed)
      .map(([name]) => name);
    const relevant = report.findings.filter(({ kind }) => kinds.has(kind));
    const colorFailure = report.findings.some(({ kind }) => kind === 'red-green-only');
    const expectedColorFailure = question.id === 'color-deficient' && variant === 'before';
    if (colorFailure !== expectedColorFailure) throw new Error(`${variant} color gate: ${JSON.stringify(report.findings)}`);
    return { failed, relevant };
  });
  const [before, after] = results;
  const measured = kinds.size > 0 || Boolean(predicate);
  const counts = results.flatMap(({ relevant, failed }) => [relevant.length, failed.length]);
  const judgment = measured ? '' : ` | Judgment. ${question.look}`;
  console.log(`${question.id} | ${counts.join(' | ')}${judgment}`);
  if ((measured && !counts[0] && !counts[1]) || counts[2] || counts[3]) {
    throw new Error(`before or after failed the review contract: ${JSON.stringify({ before, after, reports: Object.values(variants).map(({ findings }) => findings) })}`);
  }
}
