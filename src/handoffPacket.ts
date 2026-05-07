import type { ApprovalCard, HandoffPacket, HealthReport, ToolCheck } from "./types";

export function buildLocalHandoffPacket(report: HealthReport | null, checks: ToolCheck[], approvalCards: ApprovalCard[]): HandoffPacket {
  const failed = checks.filter((check) => check.status !== "installed" && check.status !== "optional_skipped");
  const askInstructor = approvalCards.filter((card) => card.decision === "ask_instructor");

  return {
    generated_at: new Date().toISOString(),
    student_summary_ko: report?.summary.beginner_message ?? "진단 결과를 바탕으로 강사에게 전달할 요약을 만들었습니다.",
    instructor_summary_ko: report?.summary.instructor_message ?? `${failed.length}개 항목 확인이 필요합니다.`,
    next_action_ko: askInstructor.length > 0
      ? "강사 또는 조교가 승인 큐에서 도움 요청으로 표시된 항목을 먼저 확인하세요."
      : failed.length > 0
        ? "실패한 항목의 support_action과 redacted evidence를 확인하세요."
        : "필수 항목이 준비되었습니다.",
    checks,
    approval_cards: approvalCards,
  };
}

export function formatHandoffPacket(packet: HandoffPacket): string {
  const lines = [
    "# Vibe Coding Setup 강사용 핸드오프",
    "",
    `생성 시각: ${packet.generated_at}`,
    "",
    "## 학생용 요약",
    packet.student_summary_ko,
    "",
    "## 강사용 요약",
    packet.instructor_summary_ko,
    "",
    "## 다음 행동",
    packet.next_action_ko,
    "",
    "## 승인 큐",
    ...packet.approval_cards.map((card) => `- ${card.step.label_ko}: ${card.decision} / ${card.reason_ko}`),
    "",
    "## 진단 결과",
    ...packet.checks.map((check) => `- ${check.label}: ${check.status} / ${check.support_action}`),
  ];

  return lines.join("\n");
}
