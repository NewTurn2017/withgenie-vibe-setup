import { deriveApprovalQueue } from "../../src/approvalQueue";
import type { ApprovalDecision, CheckStatus, RecipeStep, SetupPlan, ToolCheck } from "../../src/types";

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function step(id: string, action_phase: RecipeStep["action_phase"] = "detect"): RecipeStep {
  return {
    id,
    target_os: id.includes(".windows.") ? "windows" : null,
    label_ko: id,
    description_ko: id,
    verify_command_label: id,
    required_for_class: !id.includes("install"),
    requires_consent: action_phase !== "detect",
    may_require_elevation: id.includes("winget"),
    requires_browser: action_phase === "external_flow",
    risk_tier: action_phase === "install" || action_phase === "external_flow" ? "user_mediated" : "safe",
    action_phase,
    approval_copy_ko: id,
    expected_permission_prompt_ko: id,
    package_source: null,
    rollback_note_ko: id,
    support_handoff_ko: id,
    command_preview: id,
    requires_elevation_method: "none",
    required_version_hint: null,
    docs_url: "https://example.invalid",
  };
}

const plan: SetupPlan = {
  current_os: "windows",
  forbidden_commands: [],
  security_notes: [],
  steps: [
    step("node.version"),
    step("node.install.windows.winget", "install"),
    step("npm.version"),
    step("pnpm.version"),
    step("pnpm.install.windows.npm", "install"),
    step("vercel.whoami"),
    step("vercel.install.windows.npm", "install"),
    step("windows.vcredist.x64"),
    step("windows.vcredist.install.x64.winget", "install"),
    step("windows.webview2.runtime"),
    step("windows.webview2.install.winget", "install"),
    step("codex.app.windows"),
    step("codex.app.install.windows.download", "install"),
    step("supabase.version"),
    step("supabase.auth.status"),
    step("supabase.login", "external_flow"),
    step("supabase.install.windows.standalone", "install"),
  ],
};

function check(id: string, status: CheckStatus): ToolCheck {
  return {
    id,
    label: id,
    required_for_class: true,
    status,
    detected_version: status === "installed" ? "ok" : null,
    required_version: null,
    verify_command_label: id,
    beginner_message: id,
    support_action: id,
    evidence: { exit_code: status === "installed" ? 0 : 1, duration_ms: 1, stdout_redacted: "", stderr_redacted: "" },
    links: [],
  };
}

function actionIds(statuses: Array<[string, CheckStatus]>, decisions: Record<string, ApprovalDecision> = {}) {
  return deriveApprovalQueue(plan, statuses.map(([id, status]) => check(id, status)), decisions).map((card) => [card.id, card.step.id, card.step.action_phase]);
}

{
  const ids = actionIds([
    ["node.version", "installed"],
    ["npm.version", "missing"],
    ["pnpm.version", "missing"],
    ["vercel.whoami", "missing"],
  ]);

  assert(!ids.some(([, actionId]) => actionId === "node.install.windows.winget"), "npm failure after Node is installed must not reinstall Node");
  assert(!ids.some(([, actionId]) => actionId === "pnpm.install.windows.npm"), "pnpm install must wait until npm is verified");
  assert(!ids.some(([, actionId]) => actionId === "vercel.install.windows.npm"), "vercel install must wait until npm is verified");
  assert(ids.some(([checkId, , phase]) => checkId === "npm.version" && phase === "manual_guidance"), "npm failure with installed Node should be manual guidance, not a detect action button");
}

{
  const ids = actionIds([
    ["npm.version", "installed"],
    ["pnpm.version", "missing"],
    ["vercel.whoami", "missing"],
  ]);

  assert(ids.some(([, actionId]) => actionId === "pnpm.install.windows.npm"), "pnpm should install once npm is verified");
  assert(ids.some(([, actionId]) => actionId === "vercel.install.windows.npm"), "vercel should install once npm is verified");
}

{
  const ids = actionIds([
    ["windows.vcredist.x64", "installed"],
    ["windows.webview2.runtime", "installed"],
    ["codex.app.windows", "missing"],
    ["supabase.version", "missing"],
  ]);

  assert(ids.some(([, actionId]) => actionId === "codex.app.install.windows.download"), "Codex app should open the Microsoft installer step when runtimes are ready");
  assert(ids.some(([, actionId]) => actionId === "supabase.install.windows.standalone"), "Supabase CLI should use the Windows standalone installer step when missing");
}

{
  const ids = actionIds([
    ["supabase.version", "missing"],
    ["supabase.auth.status", "missing"],
  ]);

  assert(ids.filter(([, actionId]) => actionId === "supabase.install.windows.standalone").length === 1, "Supabase auth should wait behind the CLI installer instead of adding a confusing second action");
  assert(!ids.some(([, actionId]) => actionId === "supabase.login"), "Supabase login must wait until the CLI is installed");
}

{
  const ids = actionIds([
    ["supabase.version", "installed"],
    ["supabase.auth.status", "needs_repair"],
  ]);

  assert(ids.some(([checkId, actionId, phase]) => checkId === "supabase.auth.status" && actionId === "supabase.login" && phase === "external_flow"), "Supabase auth should become a browser login action after the CLI is installed");
}

{
  const ids = actionIds([
    ["windows.vcredist.x64", "missing"],
    ["windows.webview2.runtime", "missing"],
    ["codex.app.windows", "missing"],
  ]);

  assert(ids.some(([, actionId]) => actionId === "windows.vcredist.install.x64.winget"), "Codex flow should repair VC++ runtime before Codex install");
  assert(ids.some(([, actionId]) => actionId === "windows.webview2.install.winget"), "Codex flow should repair WebView2 runtime before Codex install");
  assert(!ids.some(([, actionId]) => actionId === "codex.app.install.windows.download"), "Codex installer should wait until Windows runtimes are installed");
}

console.log("approvalQueue regression tests passed");
