import { expect, test } from "bun:test";
import { chmod, cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

const run = (cwd: string, cmd: string[], env: NodeJS.ProcessEnv) => {
  const result = Bun.spawnSync(cmd, { cwd, env, stdout: "pipe", stderr: "pipe" });
  return {
    code: result.exitCode,
    stdout: result.stdout.toString(),
    stderr: result.stderr.toString(),
  };
};

const git = (cwd: string, ...args: string[]) => {
  const result = run(cwd, ["git", ...args], process.env);
  expect(result.code).toBe(0);
  return result.stdout.trim();
};

const withRepository = async (action: (cwd: string) => Promise<void>) => {
  const temporaryDirectory = tmpdir();
  const prefix = join(temporaryDirectory, "agent-hooks-release-");
  const cwd = await mkdtemp(prefix);
  const libraryDirectory = join(cwd, "scripts/lib");
  const deleteSource = new URL("release-delete.ts", import.meta.url);
  const deleteDestination = join(cwd, "scripts/release-delete.ts");
  const loggerSource = new URL("lib/log.ts", import.meta.url);
  const loggerDestination = join(cwd, "scripts/lib/log.ts");
  const justfileSource = new URL("../justfile", import.meta.url);
  const justfileDestination = join(cwd, "justfile");
  const mockPath = join(cwd, "mock.ts");
  const sandboxPath = join(cwd, "sandbox");
  const requestsPath = join(cwd, "requests");
  const serializedMockPath = JSON.stringify(mockPath);
  try {
    await mkdir(libraryDirectory, { recursive: true });
    await cp(deleteSource, deleteDestination);
    await cp(loggerSource, loggerDestination);
    await cp(justfileSource, justfileDestination);
    await writeFile(mockPath, `
const requests = process.env.REQUESTS;
if (!requests) throw new Error("missing requests path");
globalThis.fetch = async (input, init) => {
  const url = String(input);
  const method = init === undefined || init.method === undefined ? "GET" : init.method;
  await Bun.write(requests, (await Bun.file(requests).text()) + method + " " + url + "\\n");
  if (process.env.FAIL_API === "yes") return new Response("mock failure", {status: 403});
  if (method === "DELETE") return new Response(null, {status: 204});
  return Response.json({id: 42});
};
`);
    await writeFile(sandboxPath, `#!${process.execPath}\nconst result = Bun.spawnSync([process.execPath, "--preload", ${serializedMockPath}, ...process.argv.slice(3)], {stdout: "inherit", stderr: "inherit"});\nprocess.exit(result.exitCode);\n`);
    await chmod(sandboxPath, 0o755);
    await writeFile(requestsPath, "");
    git(cwd, "init");
    git(cwd, "remote", "add", "origin", "https://codeberg.org/example/disposable.git");
    git(cwd, "-c", "user.name=Test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false", "commit", "--allow-empty", "--message", "fixture");
    await action(cwd);
  } finally {
    await rm(cwd, { recursive: true, force: true });
  }
};

const deleteRelease = (cwd: string, failApi: boolean) => run(cwd, ["just", "release-delete", "v1.0.0"], {
  ...process.env,
  PATH: `${cwd}:${process.env.PATH}`,
  CODEBERG_TOKEN: "mock-token",
  REQUESTS: join(cwd, "requests"),
  FAIL_API: failApi ? "yes" : "no",
});

test("local tag failure prevents all API calls", async () => {
  await withRepository(async (cwd) => {
    const result = deleteRelease(cwd, false);
    expect(result.code).not.toBe(0);
    const requestsPath = join(cwd, "requests");
    const requests = await readFile(requestsPath, "utf8");
    expect(requests).toBe("");
  });
});

test("deletes local tag before release and remote tag", async () => {
  await withRepository(async (cwd) => {
    git(cwd, "tag", "v1.0.0");
    const result = deleteRelease(cwd, false);
    expect(result.code).toBe(0);
    const tags = git(cwd, "tag", "--list");
    expect(tags).toBe("");
    const requestsPath = join(cwd, "requests");
    const requests = await readFile(requestsPath, "utf8");
    const requestLines = requests.trim().split("\n");
    expect(requestLines).toEqual([
      "GET https://codeberg.org/api/v1/repos/example/disposable/releases/tags/v1.0.0",
      "DELETE https://codeberg.org/api/v1/repos/example/disposable/releases/42",
      "GET https://codeberg.org/api/v1/repos/example/disposable/tags/v1.0.0",
      "DELETE https://codeberg.org/api/v1/repos/example/disposable/tags/v1.0.0",
    ]);
  });
});

test("API failure remains visible after local deletion", async () => {
  await withRepository(async (cwd) => {
    git(cwd, "tag", "v1.0.0");
    const result = deleteRelease(cwd, true);
    expect(result.code).not.toBe(0);
    const tags = git(cwd, "tag", "--list");
    expect(tags).toBe("");
    expect(result.stderr).toContain("check release: 403 mock failure");
  });
});
