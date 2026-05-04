import type { ApprovalDecision, ElevationMethod, RiskTier } from "./types";

export const riskTierLabels: Record<RiskTier, string> = {
  safe: "안전 진단",
  user_mediated: "사용자 승인",
  permission_prompt: "권한 확인 가능",
  blocked: "자동 실행 안 함",
};

export const riskTierDescriptions: Record<RiskTier, string> = {
  safe: "현재 상태만 확인하고 컴퓨터 설정을 바꾸지 않습니다.",
  user_mediated: "브라우저 로그인이나 업데이트처럼 사용자가 직접 승인해야 합니다.",
  permission_prompt: "설치 도구나 운영체제 권한 창이 나타날 수 있습니다.",
  blocked: "비밀번호, 토큰, 임의 명령처럼 앱이 실행하지 않는 흐름입니다.",
};

export const approvalDecisionLabels: Record<ApprovalDecision, string> = {
  pending: "대기",
  approved: "실행 요청됨",
  deferred: "나중에",
  manual: "직접 진행",
  ask_instructor: "강사 도움 요청",
};

export const elevationMethodLabels: Record<ElevationMethod, string> = {
  none: "권한 상승 없음",
  osascript_admin: "macOS 관리자 권한 창",
  windows_runas: "Windows UAC 권한 창",
  user_managed: "사용자가 직접 처리",
};

export function riskTierClassName(riskTier: RiskTier): string {
  return `risk-${riskTier.replace(/_/g, "-")}`;
}
