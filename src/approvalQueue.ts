import type { ApprovalCard, ApprovalDecision, RecipeStep, SetupPlan, ToolCheck } from "./types";

const actionStatuses = new Set<ToolCheck["status"]>([
  "missing",
  "needs_repair",
  "needs_restart",
  "blocked",
  "unsupported",
]);

export function deriveApprovalQueue(
  plan: SetupPlan | null,
  checks: ToolCheck[],
  decisions: Record<string, ApprovalDecision>,
): ApprovalCard[] {
  if (!plan) {
    return [];
  }

  const stepsById = new Map<string, RecipeStep>(plan.steps.map((step) => [step.id, step]));
  const checksById = new Map<string, ToolCheck>(checks.map((check) => [check.id, check]));
  const seenActionIds = new Set<string>();
  const cards: ApprovalCard[] = [];

  for (const check of checks.filter((item) => item.required_for_class && actionStatuses.has(item.status))) {
    const step = resolveActionStep(plan, stepsById, checksById, check) ?? fallbackStep(check);
    if (seenActionIds.has(step.id)) {
      continue;
    }
    seenActionIds.add(step.id);
    cards.push({
      id: check.id,
      step,
      check,
      decision: decisions[check.id] ?? "pending",
      reason_ko: approvalReason(check.status),
    });
  }

  return cards;
}

const installActionByCheckId: Record<string, string> = {
  "node.version": "node.install.windows.winget",
  "pnpm.version": "pnpm.install.windows.npm",
  "git.version": "git.install.windows.winget",
  "windows.vcredist.x64": "windows.vcredist.install.x64.winget",
  "windows.webview2.runtime": "windows.webview2.install.winget",
};

function isInstalled(check: ToolCheck | undefined): boolean {
  return check?.status === "installed";
}

function actionIdForCheck(
  check: ToolCheck,
  checksById: Map<string, ToolCheck>,
  currentOs: SetupPlan["current_os"],
): string | undefined {
  if (check.id === "npm.version") {
    return isInstalled(checksById.get("node.version")) ? undefined : "node.install.windows.winget";
  }

  if (check.id === "pnpm.version" && !isInstalled(checksById.get("npm.version"))) {
    return undefined;
  }

  if (check.id === "gh.auth.status") {
    return check.status === "missing" ? "gh.install.windows.winget" : "gh.auth.login";
  }

  if (check.id === "vercel.whoami") {
    if (!isInstalled(checksById.get("npm.version"))) {
      return undefined;
    }
    return check.status === "missing" ? "vercel.install.windows.npm" : "vercel.login";
  }

  if (check.id === "codex.app.windows") {
    if (currentOs === "windows" && !isInstalled(checksById.get("windows.vcredist.x64"))) {
      return "windows.vcredist.install.x64.winget";
    }
    if (currentOs === "windows" && !isInstalled(checksById.get("windows.webview2.runtime"))) {
      return "windows.webview2.install.winget";
    }
    return "codex.app.install.windows.download";
  }

  if (check.id === "supabase.version") {
    if (currentOs === "windows") {
      return "supabase.install.windows.standalone";
    }
    if (currentOs === "macos" && isInstalled(checksById.get("brew.version"))) {
      return "supabase.install.macos.brew";
    }
    return undefined;
  }

  return installActionByCheckId[check.id];
}

function resolveActionStep(
  plan: SetupPlan,
  stepsById: Map<string, RecipeStep>,
  checksById: Map<string, ToolCheck>,
  check: ToolCheck,
): RecipeStep | undefined {
  const installActionId = actionIdForCheck(check, checksById, plan.current_os);
  const installStep = installActionId ? stepsById.get(installActionId) : undefined;
  if (installStep && (!installStep.target_os || installStep.target_os === plan.current_os)) {
    return installStep;
  }

  // If a failing diagnostic does not have a safe executable action for the
  // current dependency state, do not expose the diagnostic recipe itself as a
  // fake action. Falling back keeps the card in manual guidance mode.
  return undefined;
}

function approvalReason(status: ToolCheck["status"]): string {
  switch (status) {
    case "missing":
      return "필수 도구가 설치되어 있지 않아 다음 행동을 선택해야 합니다.";
    case "needs_repair":
      return "도구는 보이지만 버전, PATH, 로그인, 네트워크 상태를 다시 확인해야 합니다.";
    case "needs_restart":
      return "설치 또는 설정 반영을 위해 재시작이나 새 터미널 확인이 필요합니다.";
    case "blocked":
      return "권한, 정책, 네트워크 문제로 학생 혼자 해결하기 어려울 수 있습니다.";
    case "unsupported":
      return "현재 운영체제나 정책에서는 자동 해결이 어렵습니다.";
    default:
      return "추가 확인이 필요합니다.";
  }
}

function fallbackStep(check: ToolCheck): RecipeStep {
  const isHardBlocked = check.status === "blocked" || check.status === "unsupported";
  return {
    id: check.id,
    label_ko: check.label,
    description_ko: check.beginner_message,
    verify_command_label: check.verify_command_label,
    required_for_class: check.required_for_class,
    requires_consent: true,
    may_require_elevation: isHardBlocked,
    requires_browser: false,
    risk_tier: isHardBlocked ? "blocked" : "user_mediated",
    action_phase: isHardBlocked ? "not_automated" : "manual_guidance",
    approval_copy_ko: "현재 항목은 수동 확인 또는 강사 도움이 필요합니다.",
    expected_permission_prompt_ko: "권한 창이 나타날 수 있습니다.",
    package_source: check.links[0] ?? null,
    rollback_note_ko: "자동 되돌리기는 제공하지 않습니다. 변경 전 안내를 확인하세요.",
    support_handoff_ko: check.support_action,
    command_preview: check.verify_command_label,
    requires_elevation_method: "user_managed",
    required_version_hint: check.required_version ?? null,
    docs_url: check.links[0] ?? "https://github.com/NewTurn2017/withgenie-vibe-setup",
  };
}
