import { accessSync, constants, readdirSync } from "fs";
import { homedir } from "os";
import { join } from "path";
import { log } from "./lib/log.ts";

const home = homedir();

type Result = { pass: boolean; msg: string };

const checkBlocked = (path: string): Result => {
  try {
    accessSync(path, constants.R_OK);
    return { pass: false, msg: `${path} — readable` };
  } catch {
    return { pass: true, msg: `${path} — access denied` };
  }
};

const checkSshDir = (): Result => {
  const sshDir = join(home, ".ssh");
  try {
    const files = readdirSync(sshDir);
    const unexpected = files.filter((f) => f !== "known_hosts");
    if (unexpected.length > 0) {
      return {
        pass: false,
        msg: `${sshDir} — unexpected files: ${unexpected.join(", ")}`,
      };
    }
    return { pass: true, msg: `${sshDir} — only known_hosts` };
  } catch {
    return { pass: true, msg: `${sshDir} — access denied` };
  }
};

const blockedPaths = [
  join(home, ".config/sops"),
  "/var/lib/sops-nix/key.txt",
  "/etc/initrd-host-key",
  "/run/secrets",
  "/etc/shadow",
  "/etc/passwd",
  "/etc/ssh",
  "/root",
  "/mnt/backup",
];

const results = [checkSshDir(), ...blockedPaths.map(checkBlocked)];
results.forEach((r) => {
  if (r.pass) {
    log.ok(r.msg);
  } else {
    log.error(r.msg);
  }
});

const failures = results.filter((r) => !r.pass);
if (failures.length > 0) {
  process.exit(1);
}
