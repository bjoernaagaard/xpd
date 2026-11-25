import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

import { editUrl, firstSentence, page, sectionsText, yaml } from '../src/markdown.mjs';
import { cell, code, readAuthored, reflow, text, validateNames } from './reference-common.mjs';

function typeOf(schema) {
  if (!schema) return 'any';
  if (schema.anyOf) {
    const kinds = schema.anyOf.map(typeOf);
    return [...new Set(kinds)].join(' or ');
  }
  if (schema.enum) return schema.enum.map((value) => code(JSON.stringify(value))).join(', ');
  if (schema.const !== undefined) return code(JSON.stringify(schema.const));
  if (schema.type === 'array') return `list of ${typeOf(schema.items)}`;
  if (Array.isArray(schema.type)) return schema.type.join(' or ');
  return schema.type ?? 'any';
}

function objectsIn(schema) {
  if (!schema) return [];
  if (schema.anyOf) return schema.anyOf.flatMap(objectsIn);
  if (schema.type === 'array') return objectsIn(schema.items);
  return schema.properties ? [schema] : [];
}

function fieldRows(schema, prefix) {
  const rows = [];
  for (const object of objectsIn(schema)) {
    const list = schema.type === 'array' || schema.anyOf?.some((alt) => alt.type === 'array');
    const base = list ? `${prefix}[]` : prefix;
    const required = new Set(object.required ?? []);
    for (const [name, field] of Object.entries(object.properties)) {
      const path = `${base}.${name}`;
      rows.push({ path, field, required: required.has(name) });
      rows.push(...fieldRows(field, path));
    }
  }
  return rows;
}

function fieldTable(rows, summary) {
  const table = [
    '| Field | Type | Required | Default | Description |',
    '| --- | --- | --- | --- | --- |',
    ...rows.map(
      ({ path, field, required }) =>
        `| ${code(path)} | ${cell(typeOf(field))} | ${required ? 'yes' : 'no'} | ${'default' in field ? code(JSON.stringify(field.default)) : ' '} | ${cell(field.description) || ' '} |`,
    ),
  ];
  return rows.length > 12 ? `<details>\n<summary>${summary}</summary>\n\n${table.join('\n')}\n\n</details>` : table.join('\n');
}

function signature(tool) {
  const schema = tool.inputSchema;
  const required = new Set(schema.required ?? []);
  const args = Object.entries(schema.properties ?? {}).map(([name, field]) =>
    required.has(name) ? name : `${name}=${JSON.stringify(field.default ?? null)}`,
  );
  return `${tool.name}(${args.join(', ')})`;
}

const BEHAVIOR = [
  ['readOnlyHint', 'Changes no file', 'May change files'],
  ['destructiveHint', 'May overwrite what is there', 'Only adds to what is there'],
  ['idempotentHint', 'A second call with the same arguments changes nothing more', 'A second call may do more'],
  ['openWorldHint', 'Runs code that may reach outside the project', 'Stays inside the project'],
];

function toolPage(tool, prose) {
  const schema = tool.inputSchema;
  const required = new Set(schema.required ?? []);
  const body = [prose.intro, '## Call', '', '```python', signature(tool), '```'];
  const annotations = tool.annotations ?? {};
  const behavior = BEHAVIOR
    .filter(([key]) => key in annotations && !(annotations.readOnlyHint && key === 'destructiveHint'))
    .map(([key, yes, no]) => (annotations[key] ? yes : no));
  if (behavior.length) body.push('', ...behavior.map((line) => `- ${line}`));
  const tags = tool._meta?.fastmcp?.tags ?? [];
  if (tags.length) body.push('', `Tags: ${tags.map(code).join(', ')}.`);
  body.push('', '## Description', '', 'The server gives the agent this description of the tool.', '', reflow(tool.description));
  const args = Object.entries(schema.properties ?? {});
  body.push('', '## Arguments', '');
  if (!args.length) body.push('It takes no arguments.');
  for (const [name, field] of args) {
    body.push(`### ${text(name)}`, '', `Type: ${typeOf(field)}. ${required.has(name) ? 'Required.' : `Optional, default ${code(JSON.stringify(field.default ?? null))}.`}`);
    if (field.description) body.push('', reflow(field.description));
    const rows = fieldRows(field, name);
    if (rows.length) body.push('', fieldTable(rows, `The ${rows.length} fields of ${code(name)}`));
    body.push('');
  }
  body.push('## Result', '');
  if (tool.outputSchema) {
    if (tool.outputSchema.description) body.push(reflow(tool.outputSchema.description), '');
    const rows = fieldRows(tool.outputSchema, 'result').map((row) => ({ ...row, path: row.path.replace(/^result\.?/, '') }));
    body.push(rows.length ? fieldTable(rows, `The ${rows.length} fields of the result`) : `Type: ${typeOf(tool.outputSchema)}.`);
  } else {
    body.push('The tool declares no output schema.');
  }
  body.push('', sectionsText(prose.sections));
  return body.join('\n');
}

export async function generateMcpReference(mcp, { repository, mcpFolder, mcpOut, checkExample }) {
  rmSync(mcpOut, { recursive: true, force: true });
  mkdirSync(join(mcpOut, 'tools'), { recursive: true });
  const toolNames = mcp.tools.map((tool) => tool.name);
  validateNames(repository, 'tool', join(mcpFolder, 'tools'), toolNames);
  validateNames(repository, 'page', mcpFolder, ['index', 'resources', 'prompts', ...toolNames.map((name) => `tools/${name}`)]);

  for (const tool of mcp.tools) {
    const source = join(mcpFolder, 'tools', `${tool.name}.md`);
    const prose = readAuthored(source);
    if (!prose.sections.some((section) => section.heading === 'Errors')) {
      throw new Error(`${relative(repository, source)} needs an "## Errors" section`);
    }
    await checkExample(source, prose.source, tool.name);
    writeFileSync(
      join(mcpOut, 'tools', `${tool.name}.md`),
      page(
        {
          title: yaml(`${tool.name} tool`),
          description: yaml(`The ${tool.name} tool of the xpd MCP server. ${firstSentence(tool.description)}`),
          editUrl: yaml(editUrl(`website/reference/mcp/tools/${tool.name}.md`)),
          sidebar: { label: yaml(tool.name) },
          tableOfContents: { maxHeadingLevel: 3 },
        },
        toolPage(tool, prose),
      ),
    );
  }

  function namedSections(what, source, items, render) {
    const prose = readAuthored(source);
    const names = items.map((item) => item.key);
    const headings = prose.sections.map((section) => section.heading);
    const extra = headings.filter((heading) => !names.includes(heading) && !heading.startsWith('About '));
    const missing = names.filter((name) => !headings.includes(name));
    if (missing.length || extra.length) {
      throw new Error(
        `${relative(repository, source)} needs a "## <name>" section for each ${what}, and "## About ..." for others. ` +
          `Missing: ${missing.join(', ') || 'none'}. Naming no ${what}: ${extra.join(', ') || 'none'}.`,
      );
    }
    const byHeading = new Map(prose.sections.map((section) => [section.heading, section.body]));
    const sections = items.map((item) => [`## ${item.heading}`, '', render(item), '', byHeading.get(item.key)].join('\n'));
    const about = prose.sections.filter((section) => section.heading.startsWith('About ')).map((section) => `## ${section.heading.slice(6)}\n\n${section.body}`);
    return [prose.intro, ...sections, ...about].join('\n\n');
  }

  {
    const groupOf = (uri) =>
      uri.startsWith('skill://') ? `skill://${uri.split('/')[2]}/` : uri.startsWith('ui://') ? 'ui://' : uri;
    const groups = new Map();
    for (const resource of mcp.resources) {
      const key = groupOf(resource.uri);
      groups.set(key, [...(groups.get(key) ?? []), resource]);
    }
    const resourceItems = [...groups].map(([key, resources]) => ({ key, heading: text(key), resources, template: false }));
    const templateItems = mcp.resource_templates.map((template) => ({ key: template.uriTemplate, heading: text(template.uriTemplate), resources: [template], template: true }));
    const render = (item) => {
      const lines = [];
      if (item.template) lines.push('A resource template. Fill in the parts in braces to read one resource.', '');
      lines.push('| URI | Name | Type | Description |', '| --- | --- | --- | --- |');
      for (const resource of item.resources) {
        lines.push(`| ${code(resource.uri ?? resource.uriTemplate)} | ${text(resource.name)} | ${code(resource.mimeType ?? 'unknown')} | ${cell(resource.description) || ' '} |`);
      }
      const audience = item.resources[0].annotations?.audience;
      if (audience) lines.push('', `Meant for the ${audience.map((who) => (who === 'assistant' ? 'agent' : who)).join(' and the ')}.`);
      return lines.join('\n');
    };
    const body = namedSections('resource or template', join(mcpFolder, 'resources.md'), [...resourceItems, ...templateItems], render);
    writeFileSync(
      join(mcpOut, 'resources.md'),
      page(
        {
          title: yaml('MCP resources'),
          description: yaml(`The ${mcp.resources.length} resources and ${mcp.resource_templates.length} resource templates of the xpd MCP server.`),
          editUrl: yaml(editUrl('website/reference/mcp/resources.md')),
          sidebar: { label: yaml('Resources'), order: 1 },
        },
        `The server lists ${mcp.resources.length} resources and ${mcp.resource_templates.length} resource templates.\n\n${body}`,
      ),
    );
  }

  {
    const items = mcp.prompts.map((prompt) => ({ key: prompt.name, heading: text(prompt.name), prompt }));
    const render = ({ prompt }) => {
      const lines = [reflow(prompt.description), ''];
      const args = prompt.arguments ?? [];
      if (args.length) {
        lines.push('| Argument | Required | Description |', '| --- | --- | --- |');
        lines.push(...args.map((arg) => `| ${code(arg.name)} | ${arg.required ? 'yes' : 'no'} | ${cell(arg.description) || ' '} |`));
      } else {
        lines.push('It takes no arguments.');
      }
      return lines.join('\n');
    };
    const body = namedSections('prompt', join(mcpFolder, 'prompts.md'), items, render);
    writeFileSync(
      join(mcpOut, 'prompts.md'),
      page(
        {
          title: yaml('MCP prompts'),
          description: yaml(`The ${mcp.prompts.length} prompts of the xpd MCP server, with their arguments.`),
          editUrl: yaml(editUrl('website/reference/mcp/prompts.md')),
          sidebar: { label: yaml('Prompts'), order: 2 },
        },
        `The server lists ${mcp.prompts.length} prompts.\n\n${body}`,
      ),
    );
  }

  {
    const prose = readAuthored(join(mcpFolder, 'index.md'));
    const behavior = (tool) => (tool.annotations?.readOnlyHint ? 'No' : 'Yes');
    const body = [
      prose.intro,
      '',
      `The server is named ${code(mcp.server.name)}, and xpd ${mcp.server.version} lists ${mcp.tools.length} tools, ${mcp.resources.length} resources, ${mcp.resource_templates.length} resource templates and ${mcp.prompts.length} prompts.`,
      '',
      '## Tools',
      '',
      '| Tool | What it does | May change files |',
      '| --- | --- | --- |',
      ...mcp.tools.map((tool) => `| [${code(tool.name)}](./tools/${tool.name}/) | ${text(firstSentence(tool.description))} | ${behavior(tool)} |`),
      '',
      'The [resources](./resources/) page lists the resources and templates, and the [prompts](./prompts/) page lists the prompts.',
      '',
      '## Instructions',
      '',
      'The server gives the agent these instructions when it connects.',
      '',
      '```text',
      mcp.instructions.trim(),
      '```',
      '',
      sectionsText(prose.sections),
    ];
    writeFileSync(
      join(mcpOut, 'index.md'),
      page(
        {
          title: yaml('MCP server reference'),
          description: yaml(`The tools, resources and prompts of the xpd MCP server, as xpd ${mcp.server.version} lists them.`),
          editUrl: yaml(editUrl('website/reference/mcp/index.md')),
          sidebar: { label: yaml('Overview'), order: 0 },
        },
        body.join('\n'),
      ),
    );
  }
}
