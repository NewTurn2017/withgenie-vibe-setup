import { execFileSync } from "node:child_process";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const outDir = join(tmpdir(), "vibe-setup-approval-test");
rmSync(outDir, { recursive: true, force: true });

execFileSync(
  process.platform === "win32" ? "npx.cmd" : "npx",
  [
    "tsc",
    "--target",
    "ES2020",
    "--module",
    "commonjs",
    "--moduleResolution",
    "node",
    "--strict",
    "--skipLibCheck",
    "--outDir",
    outDir,
    "scripts/tests/approvalQueue.test.ts",
    "src/approvalQueue.ts",
    "src/types.ts",
  ],
  { stdio: "inherit" },
);

execFileSync(
  process.execPath,
  [join(outDir, "scripts", "tests", "approvalQueue.test.js")],
  { stdio: "inherit" },
);
