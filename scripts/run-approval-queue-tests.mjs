import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const require = createRequire(import.meta.url);
const outDir = join(tmpdir(), "vibe-setup-approval-test");
const tscBin = require.resolve("typescript/bin/tsc");
rmSync(outDir, { recursive: true, force: true });

execFileSync(
  process.execPath,
  [
    tscBin,
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
