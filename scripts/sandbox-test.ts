import { accessSync, constants, readdirSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { homedir } from "node:os";
import { log } from "./log";

type Result = { pass: boolean; msg: string };

const home = homedir();
const projectDir = dirname(import.meta.dir);
const projectName = basename(projectDir);
const projectsDir = dirname(projectDir);

const checkBlocked = (path: string): Result => {
  try {
    accessSync(path, constants.R_OK);
    return { pass: false, msg: `${path} - readable` };
  } catch {
    return { pass: true, msg: `${path} - access denied` };
  }
};

const checkDirAllowlist = (path: string, allowed: string[]): Result => {
  try {
    const entries = readdirSync(path);
    const leaked = entries.filter((entry) => !allowed.includes(entry));
    if (leaked.length > 0) {
      return { pass: false, msg: `${path} - leaked: ${leaked.join(", ")}` };
    }

    const label =
      allowed.length === 0 ? "empty" : `only ${allowed.join(", ")} visible`;
    return { pass: true, msg: `${path} - ${label}` };
  } catch {
    return { pass: true, msg: `${path} - access denied` };
  }
};

const blockedPaths = [
  join(home, ".config", "sops"),
  join(home, ".ssh", "id_ed25519"),
  join(home, ".ssh", "id_rsa"),
  "/etc/shadow",
  "/root",
];

const sshDir = join(home, ".ssh");
const results = [
  checkDirAllowlist(sshDir, ["known_hosts"]),
  checkDirAllowlist(projectsDir, [projectName]),
  ...blockedPaths.map(checkBlocked),
];

results.forEach((result) => {
  if (result.pass) {
    log.ok(result.msg);
  } else {
    log.error(result.msg);
  }
});

if (results.some((result) => !result.pass)) {
  process.exit(1);
}
