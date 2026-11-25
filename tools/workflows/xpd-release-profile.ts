import { execFile } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const { workflow, step, title } = T3;
const exec = promisify(execFile);
const project = join(homedir(), "src/xpd");

function collect(name, args, input = "") {
  return step(
    name,
    { returns: Schema.Unknown },
    Effect.tryPromise(async () => {
      const pending = exec("node", ["tools/release-profile.mjs", ...args], {
        cwd: project,
        maxBuffer: 32 * 1024 * 1024,
      });
      pending.child.stdin.end(input);
      return JSON.parse((await pending).stdout);
    }),
  );
}

export default workflow(
  "xpd-release-profile",
  {
    version: 2,
    payload: Schema.Struct({
      requests: Schema.optional(Schema.Record(Schema.String, Schema.String)),
    }),
  },
  Effect.fn(function* (payload, run) {
    const releases = yield* collect("published-releases", ["list"]);
    yield* title(`Profile ten xpd releases through ${releases[0].tagName}`);
    const rows = [];
    for (const [i, release] of releases.slice(0, 10).entries()) {
      rows.push(
        yield* collect(release.tagName, [
          "profile",
          release.tagName,
          releases[i + 1]?.publishedAt ?? "",
          release.publishedAt,
          payload.requests?.[release.tagName] ?? "",
        ]),
      );
    }
    return yield* collect(
      "decision-report",
      ["report", run.id],
      JSON.stringify(rows),
    );
  }),
);
