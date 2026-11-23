import { mkdir, writeFile, rename } from "node:fs/promises";
import { text } from "node:stream/consumers";
import { execFile } from "node:child_process";
import { DatabaseSync } from "node:sqlite";
import { homedir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const exec = promisify(execFile);
const repo = "bjoernaagaard/xpd";
async function run(cmd, ...args) {
  const { stdout } = await exec(cmd, args, { maxBuffer: 80 * 1024 * 1024 });
  return stdout.trimEnd();
}
const gh = (...args) => run("gh", ...args, "--repo", repo);
const api = async (path) =>
  JSON.parse(await run("gh", "api", `repos/${repo}/${path}`));
const sec = (a, b) => Math.round((Date.parse(b) - Date.parse(a)) / 1000);
const fields =
  "databaseId,attempt,createdAt,updatedAt,conclusion,headBranch,headSha,url";
const median = (xs) => {
  const sorted = xs.toSorted((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
};

function trace(previous, ready, tag, requestId) {
  const db = new DatabaseSync(join(homedir(), ".t3/userdata/statev2.sqlite"), {
    readOnly: true,
  });
  try {
    const messages = db
      .prepare(
        `
      SELECT m.created_at AS at, m.thread_id AS thread, m.message_id AS id,
        json_extract(m.payload_json,'$.text') AS text
      FROM orchestration_v2_projection_messages m JOIN orchestration_v2_projection_threads t USING(thread_id)
      JOIN projection_projects p USING(project_id)
      WHERE p.workspace_root=? AND m.role='user' AND json_extract(m.payload_json,'$.createdBy')='user'
        AND m.created_at>? AND m.created_at<? ORDER BY m.created_at
    `,
      )
      .all(process.cwd(), previous, ready);
    const candidates = messages.filter(
      (m) =>
        /release|publish|publication|pypi/i.test(m.text) &&
        !/^(# AGENTS|<|\$bmode)/.test(m.text) &&
        m.text.length < 3000,
    );
    const request = messages.find((m) => m.id === requestId);
    if (requestId && !request)
      throw Error("Cannot verify supplied release request " + requestId);
    const commands = new Map();
    for (const thread of new Set(request ? [request.thread] : [])) {
      const start = request.at;
      const rows = db
        .prepare(
          `SELECT json_extract(payload_json,'$.input.command') AS cmd
        FROM orchestration_v2_projection_turn_items WHERE thread_id=? AND updated_at BETWEEN ? AND ?
        AND type='dynamic_tool'`,
        )
        .all(thread, start, ready);
      for (const { cmd } of rows) {
        if (typeof cmd !== "string") continue;
        for (const match of cmd.matchAll(
          /\b(gh run (?:list|view|watch)|git (?:status|push|tag)|gh release view)\b/g,
        ))
          commands.set(match[1], (commands.get(match[1]) ?? 0) + 1);
      }
    }
    return {
      request: request
        ? {
            at: request.at,
            id: request.id,
            thread: request.thread,
            toReady: sec(request.at, ready),
          }
        : null,
      candidates: candidates.map(({ at, thread, id, text }) => ({
        at,
        thread,
        id,
        namesVersion: text.includes(tag.slice(1)),
      })),
      commandMentions: Object.fromEntries(commands),
    };
  } finally {
    db.close();
  }
}

async function attempts(runInfo) {
  const history = [];
  for (let i = 1; i <= runInfo.attempt; i++) {
    const jobs = await api(
      `actions/runs/${runInfo.databaseId}/attempts/${i}/jobs?per_page=100`,
    );
    if (jobs.total_count > jobs.jobs.length) throw Error("Job list truncated");
    history.push(
      jobs.jobs
        .filter((j) => j.conclusion !== "skipped")
        .map((j) => ({
          id: j.id,
          name: j.name,
          start: j.started_at,
          end: j.completed_at,
          result: j.conclusion,
          url: j.html_url,
          steps: j.steps.map((s) => ({
            name: s.name,
            start: s.started_at,
            end: s.completed_at,
          })),
        })),
    );
  }
  return history;
}

async function profile(tag, previous, published, requestId) {
  const sha = await run("git", "rev-parse", `${tag}^{commit}`);
  const parent = await run("git", "rev-parse", sha + "^");
  const [raw, nearby, diff, config] = await Promise.all([
    gh(
      "run",
      "list",
      "-w",
      "release.yml",
      "--commit",
      sha,
      "-L",
      "100",
      "--json",
      fields,
    ),
    gh(
      "run",
      "list",
      "-w",
      "release.yml",
      "-b",
      "main",
      "--created",
      previous.slice(0, 10) + ".." + published.slice(0, 10),
      "-L",
      "1000",
      "--json",
      fields,
    ),
    run("git", "diff", "-U0", parent, sha),
    run("git", "show", sha + ":.github/workflows/release.yml"),
  ]);
  if (
    !config.includes("needs: [built, publish]") ||
    !config.includes("needs: [publish]")
  )
    throw Error(`Review the publishing dependencies at ${tag}`);
  const runs = JSON.parse(raw);
  const main = runs.find(
    (r) => r.headBranch === "main" && r.conclusion === "success",
  );
  const publish = runs.find(
    (r) => r.headBranch === tag && r.conclusion === "success",
  );
  if (!main || !publish)
    throw Error(`Missing same-commit build or publication at ${tag}`);
  const other = JSON.parse(nearby);
  const before = other.filter((r) => r.createdAt < main.createdAt);
  const predecessor = before[0];
  const [builds, publication] = await Promise.all([
    attempts(main),
    attempts(publish),
  ]);
  const all = builds.flat();
  const successful = new Map();
  for (const job of all)
    if (job.result === "success") successful.set(job.name, job);
  const jobs = [...successful.values()].sort((a, b) =>
    b.end.localeCompare(a.end),
  );
  const last = jobs[0];
  const first = all.map((j) => j.start).sort()[0];
  const end = publication
    .flat()
    .filter(
      (j) =>
        ["Install from PyPI", "GitHub Release"].includes(j.name) &&
        j.result === "success",
    );
  if (new Set(end.map((j) => j.name)).size !== 2)
    throw Error("Missing publication proof " + tag);
  const ready = end
    .map((j) => j.end)
    .sort()
    .at(-1);
  const log = (
    await gh(
      "run",
      "view",
      String(main.databaseId),
      "--job",
      String(last.id),
      "--log",
    )
  ).replace(/\x1b\[[0-9;]*m/g, "");
  const lines = diff.split("\n").filter((s) => /^[+-](?![+-])/.test(s));
  const normalized = (sign) =>
    lines
      .filter((s) => s[0] === sign)
      .map((s) => s.slice(1).replace(/\b\d+\.\d+\.\d+\b/g, "VERSION"));
  const versionOnly =
    lines.length > 0 &&
    JSON.stringify(normalized("+")) === JSON.stringify(normalized("-")) &&
    !diff.includes("Binary files") &&
    !diff.includes("rename from") &&
    !diff.includes("new file mode");
  return {
    tag,
    sha,
    main: main.url,
    publish: publish.url,
    versionOnly,
    parentBuild: before.find((r) => r.headSha === parent)?.url ?? null,
    previousMain: predecessor ?? null,
    otherSameCommit: runs.filter((r) => ![main, publish].includes(r)),
    inWindow: before
      .filter((r) => r.createdAt > previous)
      .map((r) => ({
        url: r.url,
        result: r.conclusion,
        attempt: r.attempt,
        sha: r.headSha,
      })),
    start: main.createdAt,
    first,
    wheelsReady: last.end,
    tagStart: publish.createdAt,
    ready,
    published,
    queue: sec(main.createdAt, first),
    build: sec(first, last.end),
    handoff: sec(last.end, publish.createdAt),
    publication: sec(publish.createdAt, ready),
    total: sec(main.createdAt, ready),
    buildRunnerSeconds: all.reduce(
      (sum, j) => sum + Math.max(0, sec(j.start, j.end)),
      0,
    ),
    critical: last.name,
    criticalUrl: last.url,
    soloCeiling: sec(jobs[1].end, last.end),
    cargo:
      log.match(/Finished[^\n]{0,90}?profile[^\n]*? in ([^\r\n]+)/)?.[1] ??
      null,
    hitRate: log.match(/Cache hits rate\s+([\d.]+)\s*%/)?.[1] ?? null,
    sameVersionEarlier: await Promise.all(
      before
        .filter((r) => r.createdAt > previous)
        .map(async (r) => {
          const manifest = await run(
            "git",
            "show",
            r.headSha + ":Cargo.toml",
          ).catch(() => "");
          if (!manifest.includes('version = "' + tag.slice(1) + '"'))
            return null;
          const failures =
            r.conclusion === "failure"
              ? (await attempts(r)).flat().filter((j) => j.result === "failure")
              : [];
          return {
            url: r.url,
            sha: r.headSha,
            result: r.conclusion,
            start: r.createdAt,
            end: r.updatedAt,
            failures,
          };
        }),
    ).then((list) => list.filter(Boolean)),
    overlapping: before.filter(
      (r) => r.updatedAt > main.createdAt && r.createdAt < first,
    ),
    builds,
    publicationJobs: publication,
    trace: trace(previous, ready, tag, requestId),
  };
}

function report(rows) {
  const medianSeconds = Object.fromEntries(
    [
      "queue",
      "build",
      "handoff",
      "publication",
      "total",
      "soloCeiling",
      "buildRunnerSeconds",
    ].map((key) => [key, median(rows.map((r) => r[key]))]),
  );
  const criticalCounts = {};
  for (const r of rows)
    criticalCounts[r.critical] = (criticalCounts[r.critical] ?? 0) + 1;
  return {
    rows,
    medianSeconds,
    criticalCounts,
    versionOnlyWithParentBuild: rows
      .filter((r) => r.versionOnly && r.parentBuild)
      .map((r) => r.tag),
    experiments: [
      "Move the version edit into the final product commit when the release is already requested. Compare build count, runner seconds and request-to-install time with separate bumps.",
      "Measure concurrent wheel targets with Cargo timings. Rank changes by recomputing the maximum finish time across remaining jobs. Keep the full platform matrix and smoke checks.",
      "Record request, owner, release SHA and completion together. Verify publication using the existing fresh PyPI install and GitHub Release jobs.",
    ],
  };
}

function renderReport(rows, summary) {
  const duration = (value) =>
    Math.floor(value / 60) + "m " + String(value % 60).padStart(2, "0") + "s";
  const phases = [
    ["queue", "Before first job"],
    ["build", "Build span"],
    ["handoff", "Wait for tag"],
    ["publication", "Publish and verify"],
  ];
  const longest = Math.max(...rows.map((r) => r.total));
  const escape = (value) =>
    String(value)
      .replaceAll("&", "&amp;")
      .replaceAll('"', "&quot;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;");
  const body = rows
    .map((r) => {
      const bars = phases
        .map(
          ([key, name], i) =>
            `<i style="width:${(100 * r[key]) / longest}%;background:var(--chart-${i + 1},${colors[i]})" title="${name} · ${duration(r[key])}"></i>`,
        )
        .join("");
      return `<tr><td><a href="${escape(r.main)}">${escape(r.tag)}</a></td><td><div class="bar">${bars}</div></td><td>${duration(r.total)}</td><td>${r.trace.request ? duration(r.trace.request.toReady) : "Unknown"}</td></tr>`;
    })
    .join("");
  return `<!doctype html><meta charset="utf-8"><style>
    body{font:14px/1.45 var(--font-sans,system-ui);color:var(--foreground,inherit);margin:0}h3{margin:0 0 4px}
    .legend{display:flex;flex-wrap:wrap;gap:4px 16px;font-size:12px;margin:8px 0}
    b.dot{display:inline-block;width:9px;height:9px;margin-right:4px}
    table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}td,th{text-align:right;padding:4px 2px}td:first-child,th:first-child{text-align:left}
    td:nth-child(2){width:42%}a{color:inherit}.bar{display:flex;height:12px}
    th{font-size:12px;font-weight:550}caption{text-align:left}p{margin:8px 0;font-size:12px}
    </style><h3>Time to a verified release</h3><div class="legend">${phases.map(([, name], i) => `<span><b class="dot" style="background:var(--chart-${i + 1},${colors[i]})"></b>${name}</span>`).join("")}</div>
    <table><thead><tr><th>Release</th><th>From final main run</th><th>Elapsed</th><th>From request</th></tr></thead><tbody>${body}</tbody></table>
    <p>Elapsed ends when both the fresh PyPI install and GitHub Release succeed. Request times use identified user messages. Each release links to its final main build.</p>`;
}
const colors = ["#bc8120", "#427ac2", "#8d65a7", "#468767"];

const [mode, ...args] = process.argv.slice(2);
let result;
if (mode === "list") {
  result = JSON.parse(
    await gh(
      "release",
      "list",
      "-L",
      "100",
      "--json",
      "tagName,publishedAt,isDraft",
    ),
  )
    .filter((r) => !r.isDraft)
    .sort((a, b) => b.publishedAt.localeCompare(a.publishedAt))
    .slice(0, 11);
  if (result.length < 11)
    throw Error("Need ten releases and the preceding publication boundary");
} else if (mode === "profile") result = await profile(...args);
else if (mode === "report") {
  if (!/^[a-zA-Z0-9-]+$/.test(args[0])) throw Error("Invalid run id");
  const rows = JSON.parse(await text(process.stdin));
  const { rows: evidence, ...summary } = report(rows);
  const directory = join(process.cwd(), "research", "release-profiles");
  await mkdir(directory, { recursive: true });
  const path = join(directory, args[0] + ".json");
  await writeFile(
    path + ".partial",
    JSON.stringify({ rows: evidence, ...summary }, null, 2),
  );
  await rename(path + ".partial", path);
  const chart = join(directory, args[0] + ".html");
  await writeFile(chart + ".partial", renderReport(rows, summary));
  await rename(chart + ".partial", chart);
  result = { ...summary, path, chart, releaseCount: rows.length };
} else throw Error("Expected list, profile or report");
console.log(JSON.stringify(result));
