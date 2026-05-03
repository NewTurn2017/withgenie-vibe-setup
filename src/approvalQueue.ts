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
  return checks
    .filter((check) => check.required_for_class && actionStatuses.has(check.status))
    .map((check) => {
      const step = stepsById.get(check.id) ?? fallbackStep(check);
      return {
        id: check.id,
        step,
        check,
        decision: decisions[check.id] ?? "pending",
        reason_ko: approvalReason(check.status),
      };
    });
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
