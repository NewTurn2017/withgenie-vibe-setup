export type CheckStatus =
  | "installed"
  | "missing"
  | "needs_repair"
  | "needs_restart"
  | "optional_skipped"
  | "unsupported"
  | "blocked";

export type RiskTier = "safe" | "user_mediated" | "permission_prompt" | "blocked";

export type ActionPhase = "detect" | "external_flow" | "manual_guidance" | "not_automated";

export type ElevationMethod = "none" | "osascript_admin" | "windows_runas" | "user_managed";

export type RecipeStep = {
  id: string;
  label_ko: string;
  description_ko: string;
  verify_command_label: string;
  required_for_class: boolean;
  requires_consent: boolean;
  may_require_elevation: boolean;
  requires_browser: boolean;
  risk_tier: RiskTier;
  action_phase: ActionPhase;
  approval_copy_ko: string;
  expected_permission_prompt_ko: string;
  package_source: string | null;
  rollback_note_ko: string;
  support_handoff_ko: string;
  command_preview: string;
  requires_elevation_method: ElevationMethod;
  required_version_hint?: string | null;
  docs_url: string;
};

export type SetupPlan = {
  steps: RecipeStep[];
  forbidden_commands: string[];
  security_notes: string[];
};

export type CommandEvidence = {
  exit_code?: number | null;
  duration_ms: number;
  stdout_redacted: string;
  stderr_redacted: string;
};

export type ToolCheck = {
  id: string;
  label: string;
  required_for_class: boolean;
  status: CheckStatus;
  detected_version?: string | null;
  required_version?: string | null;
  verify_command_label: string;
  beginner_message: string;
  support_action: string;
  evidence: CommandEvidence;
  links: string[];
};

export type HealthReport = {
  schema_version: string;
  generated_at: string;
  summary: {
    class_readiness: "ready_for_class" | "needs_attention" | "blocked" | "unsupported";
    required_passed: number;
    required_total: number;
    needs_instructor_help: boolean;
    beginner_message: string;
    instructor_message: string;
  };
  checks: ToolCheck[];
  redaction: {
    applied: boolean;
    rules_version: string;
    masked_fields: string[];
  };
};

export type ApprovalDecision = "pending" | "approved" | "deferred" | "manual" | "ask_instructor";

export type ApprovalCard = {
  id: string;
  step: RecipeStep;
  check: ToolCheck;
  decision: ApprovalDecision;
  reason_ko: string;
};

export type ResumeState = {
  lastUpdatedAt: string;
  lastCompletedStep: "overview" | "plan" | "diagnostics" | "approval" | "report";
  activeScreen: "overview" | "plan" | "diagnostics" | "approval" | "report" | "help";
  approvalDecisions: Record<string, ApprovalDecision>;
};

export type HandoffPacket = {
  generated_at: string;
  student_summary_ko: string;
  instructor_summary_ko: string;
  next_action_ko: string;
  checks: ToolCheck[];
  approval_cards: ApprovalCard[];
};
