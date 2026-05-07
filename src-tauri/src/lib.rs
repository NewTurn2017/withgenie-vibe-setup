use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::menu::{AboutMetadata, MenuBuilder, SubmenuBuilder};
use tauri::Emitter;
use wait_timeout::ChildExt;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const RECIPE_VERSION: &str = "2026.05.01";
const COMMAND_TIMEOUT_SECONDS: u64 = 12;
const EXECUTION_TIMEOUT_SECONDS: u64 = 20 * 60;
const NATIVE_MENU_LABELS_KO: [&str; 5] = ["파일", "편집", "보기", "창", "도움말"];
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Installed,
    Missing,
    NeedsRepair,
    NeedsRestart,
    OptionalSkipped,
    Unsupported,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RiskTier {
    Safe,
    UserMediated,
    PermissionPrompt,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ActionPhase {
    Detect,
    Install,
    ExternalFlow,
    ManualGuidance,
    NotAutomated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ElevationMethod {
    None,
    OsascriptAdmin,
    WindowsRunas,
    UserManaged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCheck {
    id: String,
    label: String,
    required_for_class: bool,
    status: CheckStatus,
    detected_version: Option<String>,
    required_version: Option<String>,
    verify_command_label: String,
    beginner_message: String,
    support_action: String,
    evidence: CommandEvidence,
    links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandEvidence {
    exit_code: Option<i32>,
    duration_ms: u128,
    stdout_redacted: String,
    stderr_redacted: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RecipeStep {
    id: &'static str,
    target_os: Option<&'static str>,
    label_ko: &'static str,
    description_ko: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    required_for_class: bool,
    requires_consent: bool,
    may_require_elevation: bool,
    requires_browser: bool,
    required_version_hint: Option<&'static str>,
    docs_url: &'static str,
    risk_tier: RiskTier,
    action_phase: ActionPhase,
    approval_copy_ko: &'static str,
    expected_permission_prompt_ko: &'static str,
    package_source: Option<&'static str>,
    rollback_note_ko: &'static str,
    support_handoff_ko: &'static str,
    command_preview: &'static str,
    requires_elevation_method: ElevationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunResult {
    command_id: String,
    command_label: String,
    status: CheckStatus,
    detected_version: Option<String>,
    beginner_message: String,
    support_action: String,
    evidence: CommandEvidence,
}

#[derive(Debug, Clone, Serialize)]
struct AppPlan {
    current_os: &'static str,
    steps: Vec<RecipeStepView>,
    forbidden_commands: Vec<&'static str>,
    security_notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct RecipeStepView {
    id: &'static str,
    target_os: Option<&'static str>,
    label_ko: &'static str,
    description_ko: &'static str,
    verify_command_label: String,
    required_for_class: bool,
    requires_consent: bool,
    may_require_elevation: bool,
    requires_browser: bool,
    required_version_hint: Option<&'static str>,
    docs_url: &'static str,
    risk_tier: RiskTier,
    action_phase: ActionPhase,
    approval_copy_ko: &'static str,
    expected_permission_prompt_ko: &'static str,
    package_source: Option<&'static str>,
    rollback_note_ko: &'static str,
    support_handoff_ko: &'static str,
    command_preview: &'static str,
    requires_elevation_method: ElevationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthReportInput {
    checks: Vec<ToolCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApprovalCardInput {
    id: String,
    label: String,
    decision: String,
    reason_ko: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandoffPacketInput {
    checks: Vec<ToolCheck>,
    approval_cards: Vec<ApprovalCardInput>,
}

#[derive(Debug, Clone, Serialize)]
struct HandoffPacket {
    generated_at: String,
    student_summary_ko: String,
    instructor_summary_ko: String,
    next_action_ko: String,
    checks: Vec<ToolCheck>,
    approval_cards: Vec<ApprovalCardInput>,
    redaction: RedactionInfo,
}

#[derive(Debug, Clone, Serialize)]
struct HealthReport {
    schema_version: &'static str,
    generated_at: String,
    app: AppInfo,
    machine: MachineInfo,
    summary: ReportSummary,
    checks: Vec<ToolCheck>,
    execution_log: Vec<LogEntry>,
    resume_state: ResumeState,
    redaction: RedactionInfo,
}

#[derive(Debug, Clone, Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    recipe_version: &'static str,
    distribution_channel: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct MachineInfo {
    os: String,
    os_version: String,
    build_number: Option<String>,
    arch: String,
    shell: String,
    network: NetworkInfo,
}

#[derive(Debug, Clone, Serialize)]
struct NetworkInfo {
    status: &'static str,
    blocked_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReportSummary {
    class_readiness: &'static str,
    required_passed: usize,
    required_total: usize,
    needs_instructor_help: bool,
    beginner_message: String,
    instructor_message: String,
}

#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    timestamp: String,
    step_id: String,
    command_label: String,
    status: &'static str,
    exit_code: Option<i32>,
    duration_ms: u128,
    message_redacted: String,
}

#[derive(Debug, Clone, Serialize)]
struct ResumeState {
    last_completed_step: &'static str,
    next_action: &'static str,
    requires_restart: bool,
}

#[derive(Debug, Clone, Serialize)]
struct RedactionInfo {
    applied: bool,
    rules_version: &'static str,
    masked_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ExecutionStatus {
    Queued,
    NeedsUserConfirm,
    Running,
    NeedsOsConsent,
    NeedsBrowserAuth,
    NeedsReboot,
    Verifying,
    Done,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecuteSetupActionInput {
    action_id: String,
    approval_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionOutcome {
    action_id: String,
    status: ExecutionStatus,
    message_ko: String,
    next_action_ko: String,
    command_preview: Option<String>,
    docs_url: Option<String>,
}

#[derive(Debug, Clone)]
struct ExternalFlowCommand {
    title_ko: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    verify_program: &'static str,
    verify_args: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
struct SetupExecutionEvent {
    action_id: String,
    status: ExecutionStatus,
    kind: &'static str,
    message_ko: String,
    command_preview: Option<String>,
    docs_url: Option<String>,
}

fn allowed_commands() -> Vec<RecipeStep> {
    vec![
        RecipeStep {
            id: "node.version",
            target_os: None,
            label_ko: "Node.js 버전 확인",
            description_ko: "수업에서 사용할 Node.js v24 LTS가 준비되어 있는지 확인합니다.",
            program: "node",
            args: &["-v"],
            required_for_class: true,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: Some("^v24."),
            docs_url: "https://nodejs.org/dist/latest-v24.x/",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "node -v",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "node.install.windows.winget",
            target_os: Some("windows"),
            label_ko: "Node.js LTS 설치",
            description_ko: "WinGet으로 수업 기준 Node.js v24 LTS 설치 관리자를 실행합니다. npm도 함께 설치됩니다.",
            program: "winget",
            args: &[
                "install",
                "--id",
                "OpenJS.NodeJS.LTS",
                "-e",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--disable-interactivity",
                "--silent",
            ],
            required_for_class: false,
            requires_consent: true,
            may_require_elevation: true,
            requires_browser: false,
            required_version_hint: Some("^v24."),
            docs_url: "https://nodejs.org/",
            risk_tier: RiskTier::PermissionPrompt,
            action_phase: ActionPhase::Install,
            approval_copy_ko: "앱이 WinGet 설치를 시작하고, 사용자는 Windows 권한 창에서 '예'만 눌러 진행합니다.",
            expected_permission_prompt_ko: "Windows 사용자 계정 컨트롤(UAC)에서 Node.js 설치 권한을 묻습니다. 자동 클릭은 하지 않습니다.",
            package_source: Some("winget: OpenJS.NodeJS.LTS"),
            rollback_note_ko: "Windows 설정 > 앱 또는 winget uninstall --id OpenJS.NodeJS.LTS 으로 제거할 수 있습니다.",
            support_handoff_ko: "UAC에서 예를 눌렀는지, winget 설치 종료 코드와 node -v 검증 결과를 확인하세요.",
            command_preview: "winget install --id OpenJS.NodeJS.LTS -e --accept-package-agreements --accept-source-agreements --disable-interactivity --silent",
            requires_elevation_method: ElevationMethod::WindowsRunas,
        },
        RecipeStep {
            id: "npm.version",
            target_os: None,
            label_ko: "npm 확인",
            description_ko: "Node.js와 함께 제공되는 npm이 실행되는지 확인합니다.",
            program: "npm",
            args: &["-v"],
            required_for_class: true,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://nodejs.org/",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "npm -v",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "pnpm.version",
            target_os: None,
            label_ko: "pnpm 확인",
            description_ko: "수업 패키지 매니저인 pnpm이 준비되어 있는지 확인합니다.",
            program: "pnpm",
            args: &["-v"],
            required_for_class: true,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://pnpm.io/",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "pnpm -v",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "pnpm.install.windows.npm",
            target_os: Some("windows"),
            label_ko: "pnpm 설치",
            description_ko: "Node.js 설치 후 npm으로 pnpm을 전역 설치합니다.",
            program: "npm",
            args: &["install", "-g", "pnpm@latest"],
            required_for_class: false,
            requires_consent: true,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://pnpm.io/installation",
            risk_tier: RiskTier::UserMediated,
            action_phase: ActionPhase::Install,
            approval_copy_ko: "Node.js/npm이 준비된 뒤 pnpm을 설치합니다. 일반적으로 Windows 권한 창은 뜨지 않습니다.",
            expected_permission_prompt_ko: "보통 권한 창이 나타나지 않습니다. Node.js 설치 직후라면 PATH 반영을 위해 재진단이 필요할 수 있습니다.",
            package_source: Some("npm: pnpm@latest"),
            rollback_note_ko: "npm uninstall -g pnpm 으로 제거할 수 있습니다.",
            support_handoff_ko: "npm 실행 가능 여부와 npm 전역 설치 로그, pnpm -v 검증 결과를 확인하세요.",
            command_preview: "npm install -g pnpm@latest",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "git.version",
            target_os: None,
            label_ko: "Git 확인",
            description_ko: "프로젝트 파일을 내려받고 기록하기 위한 Git을 확인합니다.",
            program: "git",
            args: &["--version"],
            required_for_class: true,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://github.com/git-guides/install-git",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "git --version",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "git.install.windows.winget",
            target_os: Some("windows"),
            label_ko: "Git for Windows 설치",
            description_ko: "WinGet으로 공식 Git for Windows 설치 관리자를 실행합니다. Windows 권한 창이 뜨면 사용자가 직접 '예'를 눌러야 합니다.",
            program: "winget",
            args: &[
                "install",
                "--id",
                "Git.Git",
                "-e",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--disable-interactivity",
                "--silent",
            ],
            required_for_class: false,
            requires_consent: true,
            may_require_elevation: true,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://git-scm.com/download/win",
            risk_tier: RiskTier::PermissionPrompt,
            action_phase: ActionPhase::Install,
            approval_copy_ko: "앱이 WinGet 설치를 시작하고, 사용자는 Windows 권한 창에서 '예'만 눌러 진행합니다.",
            expected_permission_prompt_ko: "Windows 사용자 계정 컨트롤(UAC)에서 Git for Windows 설치 권한을 묻습니다. 자동 클릭은 하지 않습니다.",
            package_source: Some("winget: Git.Git"),
            rollback_note_ko: "Windows 설정 > 앱 또는 winget uninstall --id Git.Git 으로 제거할 수 있습니다.",
            support_handoff_ko: "UAC에서 예를 눌렀는지, winget 설치 종료 코드와 git --version 검증 결과를 확인하세요.",
            command_preview: "winget install --id Git.Git -e --accept-package-agreements --accept-source-agreements --disable-interactivity --silent",
            requires_elevation_method: ElevationMethod::WindowsRunas,
        },
        RecipeStep {
            id: "gh.auth.status",
            target_os: None,
            label_ko: "GitHub 로그인 확인",
            description_ko: "GitHub CLI가 브라우저 로그인으로 연결되어 있는지 확인합니다.",
            program: "gh",
            args: &["auth", "status"],
            required_for_class: true,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://cli.github.com/manual/gh_auth_login",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "gh auth status",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "gh.install.windows.winget",
            target_os: Some("windows"),
            label_ko: "GitHub CLI 설치",
            description_ko: "WinGet으로 공식 GitHub CLI를 설치합니다. 로그인은 설치 후 별도 브라우저 흐름으로 진행합니다.",
            program: "winget",
            args: &[
                "install",
                "--id",
                "GitHub.cli",
                "-e",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--disable-interactivity",
                "--silent",
            ],
            required_for_class: false,
            requires_consent: true,
            may_require_elevation: true,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://cli.github.com/",
            risk_tier: RiskTier::PermissionPrompt,
            action_phase: ActionPhase::Install,
            approval_copy_ko: "앱이 WinGet 설치를 시작하고, 사용자는 Windows 권한 창에서 '예'만 눌러 진행합니다.",
            expected_permission_prompt_ko: "Windows 사용자 계정 컨트롤(UAC)에서 GitHub CLI 설치 권한을 묻습니다. 자동 클릭은 하지 않습니다.",
            package_source: Some("winget: GitHub.cli"),
            rollback_note_ko: "Windows 설정 > 앱 또는 winget uninstall --id GitHub.cli 으로 제거할 수 있습니다.",
            support_handoff_ko: "UAC에서 예를 눌렀는지, winget 설치 종료 코드와 gh --version 검증 결과를 확인하세요.",
            command_preview: "winget install --id GitHub.cli -e --accept-package-agreements --accept-source-agreements --disable-interactivity --silent",
            requires_elevation_method: ElevationMethod::WindowsRunas,
        },
        RecipeStep {
            id: "gh.auth.login",
            target_os: None,
            label_ko: "GitHub 브라우저 로그인 시작",
            description_ko: "GitHub 로그인은 브라우저에서 진행됩니다. 이 앱은 GitHub 비밀번호를 묻지 않습니다.",
            program: "gh",
            args: &[
                "auth",
                "login",
                "--web",
                "--clipboard",
                "--hostname",
                "github.com",
                "--git-protocol",
                "https",
            ],
            required_for_class: true,
            requires_consent: true,
            may_require_elevation: false,
            requires_browser: true,
            required_version_hint: None,
            docs_url: "https://cli.github.com/manual/gh_auth_login",
            risk_tier: RiskTier::UserMediated,
            action_phase: ActionPhase::ExternalFlow,
            approval_copy_ko: "공식 브라우저 로그인 흐름을 엽니다. 앱은 비밀번호나 토큰을 받지 않습니다.",
            expected_permission_prompt_ko: "브라우저 로그인 또는 device code 확인 화면이 나타날 수 있습니다.",
            package_source: None,
            rollback_note_ko: "로그인은 해당 서비스의 공식 CLI에서 관리합니다. 필요하면 공식 CLI에서 로그아웃하세요.",
            support_handoff_ko: "브라우저 로그인 단계에서 막혔는지, CLI 인증 상태가 실패했는지 확인하세요.",
            command_preview: "gh auth login --web --clipboard --hostname github.com --git-protocol https",
            requires_elevation_method: ElevationMethod::UserManaged,
        },
        RecipeStep {
            id: "vercel.whoami",
            target_os: None,
            label_ko: "Vercel 로그인 확인",
            description_ko: "Vercel CLI가 로그인되어 있는지 확인합니다.",
            program: "vercel",
            args: &["whoami"],
            required_for_class: true,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://vercel.com/docs/cli",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "vercel whoami",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "vercel.install.windows.npm",
            target_os: Some("windows"),
            label_ko: "Vercel CLI 설치",
            description_ko: "Node.js/npm이 준비된 뒤 Vercel CLI를 설치합니다. 가입/로그인은 설치 후 공식 브라우저 흐름으로 진행합니다.",
            program: "npm",
            args: &["install", "-g", "vercel@latest"],
            required_for_class: false,
            requires_consent: true,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://vercel.com/docs/cli",
            risk_tier: RiskTier::UserMediated,
            action_phase: ActionPhase::Install,
            approval_copy_ko: "npm으로 Vercel CLI를 설치합니다. 계정이 없다면 설치 후 브라우저 가입/로그인을 안내합니다.",
            expected_permission_prompt_ko: "보통 권한 창이 나타나지 않습니다. Node.js 설치 직후라면 PATH 반영을 위해 재진단이 필요할 수 있습니다.",
            package_source: Some("npm: vercel@latest"),
            rollback_note_ko: "npm uninstall -g vercel 으로 제거할 수 있습니다.",
            support_handoff_ko: "npm 실행 가능 여부와 npm 전역 설치 로그, vercel --version/whoami 검증 결과를 확인하세요.",
            command_preview: "npm install -g vercel@latest",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "vercel.login",
            target_os: None,
            label_ko: "Vercel 브라우저 로그인 시작",
            description_ko: "Vercel 로그인은 브라우저/코드 확인 흐름으로 진행됩니다. 이 앱은 Vercel 비밀번호를 묻지 않습니다.",
            program: "vercel",
            args: &["login"],
            required_for_class: true,
            requires_consent: true,
            may_require_elevation: false,
            requires_browser: true,
            required_version_hint: None,
            docs_url: "https://vercel.com/docs/cli/login",
            risk_tier: RiskTier::UserMediated,
            action_phase: ActionPhase::ExternalFlow,
            approval_copy_ko: "공식 브라우저 로그인 흐름을 엽니다. 앱은 비밀번호나 토큰을 받지 않습니다.",
            expected_permission_prompt_ko: "브라우저 로그인 또는 device code 확인 화면이 나타날 수 있습니다.",
            package_source: None,
            rollback_note_ko: "로그인은 해당 서비스의 공식 CLI에서 관리합니다. 필요하면 공식 CLI에서 로그아웃하세요.",
            support_handoff_ko: "브라우저 로그인 단계에서 막혔는지, CLI 인증 상태가 실패했는지 확인하세요.",
            command_preview: "vercel login",
            requires_elevation_method: ElevationMethod::UserManaged,
        },
        RecipeStep {
            id: "macos.version",
            target_os: None,
            label_ko: "macOS 버전 확인",
            description_ko: "Homebrew와 수업 도구를 설치할 수 있는 macOS인지 확인합니다.",
            program: "sw_vers",
            args: &["-productVersion"],
            required_for_class: cfg!(target_os = "macos"),
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: Some("14 이상"),
            docs_url: "https://docs.brew.sh/Installation",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "sw_vers -productVersion",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "macos.clt",
            target_os: None,
            label_ko: "Command Line Tools 확인",
            description_ko: "macOS 개발 도구 준비 상태를 확인합니다.",
            program: "xcode-select",
            args: &["-p"],
            required_for_class: cfg!(target_os = "macos"),
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://docs.brew.sh/Installation",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "xcode-select -p",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "brew.version",
            target_os: None,
            label_ko: "Homebrew 확인",
            description_ko: "macOS 패키지 설치 도구인 Homebrew를 확인합니다.",
            program: "brew",
            args: &["--version"],
            required_for_class: cfg!(target_os = "macos"),
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://docs.brew.sh/Installation",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "brew --version",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "windows.winget.version",
            target_os: None,
            label_ko: "WinGet 확인",
            description_ko: "Windows 패키지 설치 도구인 WinGet을 확인합니다.",
            program: "winget",
            args: &["--version"],
            required_for_class: cfg!(target_os = "windows"),
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://learn.microsoft.com/en-us/windows/package-manager/winget/",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "winget --version",
            requires_elevation_method: ElevationMethod::None,
        },
        RecipeStep {
            id: "wsl.status",
            target_os: None,
            label_ko: "WSL 확인",
            description_ko: "Windows에서 Linux 환경이 필요한 수업을 위한 WSL 상태를 확인합니다. 기본 수업에서는 선택 항목입니다.",
            program: "wsl",
            args: &["--status"],
            required_for_class: false,
            requires_consent: false,
            may_require_elevation: false,
            requires_browser: false,
            required_version_hint: None,
            docs_url: "https://learn.microsoft.com/en-us/windows/wsl/install",
            risk_tier: RiskTier::Safe,
            action_phase: ActionPhase::Detect,
            approval_copy_ko: "현재 상태만 확인하며 컴퓨터 설정을 바꾸지 않습니다.",
            expected_permission_prompt_ko: "권한 창이 나타나지 않습니다.",
            package_source: None,
            rollback_note_ko: "변경 사항이 없어 되돌릴 작업이 없습니다.",
            support_handoff_ko: "진단 결과와 redacted evidence를 강사에게 전달하세요.",
            command_preview: "wsl --status",
            requires_elevation_method: ElevationMethod::None,
        },
    ]
}

fn find_allowed_command(command_id: &str) -> Option<RecipeStep> {
    allowed_commands()
        .into_iter()
        .find(|command| command.id == command_id)
}

fn execution_outcome_for(step: &RecipeStep) -> ExecutionOutcome {
    match step.action_phase {
        ActionPhase::Install => ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::NeedsUserConfirm,
            message_ko: format!("{} 설치를 시작할 준비가 되었습니다.", step.label_ko),
            next_action_ko: step.expected_permission_prompt_ko.to_string(),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        },
        ActionPhase::ExternalFlow => ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::NeedsBrowserAuth,
            message_ko: format!("{} 단계는 공식 브라우저/CLI 로그인 흐름으로 이어집니다.", step.label_ko),
            next_action_ko: "다음 구현 슬라이스에서 앱이 공식 로그인 흐름을 열고 완료 여부를 자동 확인합니다. 현재는 비밀번호나 토큰을 앱에 입력하지 마세요.".to_string(),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        },
        ActionPhase::Detect => ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::Blocked,
            message_ko: format!("{} 항목은 현재 진단 전용 레시피입니다.", step.label_ko),
            next_action_ko: "설치/복구 레시피가 연결되기 전까지는 이 버튼이 실제 설치를 실행하지 않습니다. 자동 설치는 별도 allowlist 실행 레시피로만 추가됩니다.".to_string(),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        },
        ActionPhase::ManualGuidance | ActionPhase::NotAutomated => ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::Blocked,
            message_ko: format!("{} 항목은 자동 실행 대상이 아닙니다.", step.label_ko),
            next_action_ko: "자동화가 안전하지 않은 항목입니다. 이후 fallback에서는 앱이 명령어 복사와 올바른 셸 열기까지 도와주는 구조로 구현합니다.".to_string(),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        },
    }
}

fn emit_execution_event(app: &tauri::AppHandle, event: SetupExecutionEvent) -> Result<(), String> {
    app.emit("setup://execution-event", event)
        .map_err(|error| format!("실행 이벤트를 보낼 수 없습니다: {error}"))
}

fn execution_event_from_outcome(
    outcome: &ExecutionOutcome,
    kind: &'static str,
) -> SetupExecutionEvent {
    SetupExecutionEvent {
        action_id: outcome.action_id.clone(),
        status: outcome.status,
        kind,
        message_ko: outcome.message_ko.clone(),
        command_preview: outcome.command_preview.clone(),
        docs_url: outcome.docs_url.clone(),
    }
}

fn execution_verify_step_id(action_id: &str) -> Option<&'static str> {
    match action_id {
        "node.install.windows.winget" => Some("node.version"),
        "pnpm.install.windows.npm" => Some("pnpm.version"),
        "git.install.windows.winget" => Some("git.version"),
        _ => None,
    }
}

fn first_non_empty_line(value: &str) -> Option<String> {
    value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToString::to_string)
}

fn command_preview_for(program: &str, args: &[&str]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    }
}

fn external_flow_command_for(action_id: &str) -> Option<ExternalFlowCommand> {
    match action_id {
        "gh.auth.login" => Some(ExternalFlowCommand {
            title_ko: "GitHub 브라우저 로그인",
            program: "gh",
            args: &[
                "auth",
                "login",
                "--web",
                "--clipboard",
                "--hostname",
                "github.com",
                "--git-protocol",
                "https",
            ],
            verify_program: "gh",
            verify_args: &["auth", "status"],
        }),
        "vercel.login" => Some(ExternalFlowCommand {
            title_ko: "Vercel 브라우저 로그인",
            program: "vercel",
            args: &["login"],
            verify_program: "vercel",
            verify_args: &["whoami"],
        }),
        _ => None,
    }
}

fn execute_external_flow_action(step: &RecipeStep) -> Result<ExecutionOutcome, String> {
    let flow = external_flow_command_for(step.id)
        .ok_or_else(|| format!("외부 로그인 실행 레시피가 연결되지 않았습니다: {}", step.id))?;

    let launch_evidence = launch_external_flow_terminal(&flow);
    if launch_evidence.exit_code != Some(0) {
        let detail = first_non_empty_line(&launch_evidence.stderr_redacted)
            .or_else(|| first_non_empty_line(&launch_evidence.stdout_redacted))
            .unwrap_or_else(|| "로그인 창을 열 수 없습니다.".to_string());
        return Ok(ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::Blocked,
            message_ko: format!("{} 창을 열지 못했습니다.", step.label_ko),
            next_action_ko: format!(
                "공식 CLI 로그인 명령을 직접 실행해 주세요: {}. 오류: {detail}",
                step.command_preview
            ),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        });
    }

    Ok(ExecutionOutcome {
        action_id: step.id.to_string(),
        status: ExecutionStatus::NeedsBrowserAuth,
        message_ko: format!("{} 창을 열었습니다.", step.label_ko),
        next_action_ko: "열린 명령 프롬프트와 브라우저에서 로그인을 완료하세요. 완료 후 앱으로 돌아와 '안전 진단 시작'을 다시 누르면 로그인 상태를 확인합니다.".to_string(),
        command_preview: Some(step.command_preview.to_string()),
        docs_url: Some(step.docs_url.to_string()),
    })
}

fn launch_external_flow_terminal(flow: &ExternalFlowCommand) -> CommandEvidence {
    let start = Instant::now();
    let mut command = external_flow_launcher_command(flow);

    #[cfg(target_os = "windows")]
    if let Some(path) = refreshed_windows_path() {
        command.env("PATH", path);
    }

    match command.status() {
        Ok(status) => CommandEvidence {
            exit_code: status.code(),
            duration_ms: start.elapsed().as_millis(),
            stdout_redacted: String::new(),
            stderr_redacted: String::new(),
        },
        Err(error) => CommandEvidence {
            exit_code: None,
            duration_ms: start.elapsed().as_millis(),
            stdout_redacted: String::new(),
            stderr_redacted: redact(&error.to_string()),
        },
    }
}

fn external_flow_launcher_command(flow: &ExternalFlowCommand) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command
            .arg("/C")
            .arg("start")
            .arg(format!("위드지니 셋업 - {}", flow.title_ko))
            .arg("cmd")
            .arg("/K")
            .arg(windows_external_flow_script(flow))
            .creation_flags(CREATE_NO_WINDOW);
        return command;
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("osascript");
        let script = format!(
            "tell application \"Terminal\" to activate\ntell application \"Terminal\" to do script {}",
            applescript_string(&macos_external_flow_script(flow))
        );
        command.arg("-e").arg(script);
        return command;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let mut command = Command::new(flow.program);
        command.args(flow.args);
        command
    }
}

#[cfg(target_os = "windows")]
fn windows_external_flow_script(flow: &ExternalFlowCommand) -> String {
    format!(
        "echo 위드지니 셋업 - {} & echo. & echo 브라우저 로그인/코드 확인을 완료해 주세요. & echo 이 창은 자동으로 닫히지 않습니다. & echo. & {} & echo. & echo 로그인 명령이 끝났습니다. 검증 결과: & {} & echo. & echo 완료 후 앱으로 돌아가 '안전 진단 시작'을 다시 눌러 주세요. & pause",
        flow.title_ko,
        windows_command_line(flow.program, flow.args),
        windows_command_line(flow.verify_program, flow.verify_args),
    )
}

#[cfg(target_os = "windows")]
fn windows_command_line(program: &str, args: &[&str]) -> String {
    std::iter::once(program)
        .chain(args.iter().copied())
        .map(windows_cmd_arg)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(target_os = "windows")]
fn windows_cmd_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':'))
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

#[cfg(target_os = "macos")]
fn macos_external_flow_script(flow: &ExternalFlowCommand) -> String {
    format!(
        "echo '위드지니 셋업 - {}'; echo; echo '브라우저 로그인/코드 확인을 완료해 주세요.'; {}; echo; echo '로그인 명령이 끝났습니다. 검증 결과:'; {}; echo; echo \"완료 후 앱으로 돌아가 '안전 진단 시작'을 다시 눌러 주세요.\"",
        flow.title_ko,
        posix_command_line(flow.program, flow.args),
        posix_command_line(flow.verify_program, flow.verify_args),
    )
}

#[cfg(target_os = "macos")]
fn posix_command_line(program: &str, args: &[&str]) -> String {
    std::iter::once(program)
        .chain(args.iter().copied())
        .map(posix_shell_arg)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(target_os = "macos")]
fn posix_shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn verify_after_install(
    step: &RecipeStep,
) -> Result<(CheckStatus, Option<String>, CommandEvidence), String> {
    if step.id == "gh.install.windows.winget" {
        let evidence = run_program_with_timeout(
            "gh",
            &["--version"],
            Duration::from_secs(COMMAND_TIMEOUT_SECONDS),
        );
        if evidence.exit_code == Some(0) {
            return Ok((
                CheckStatus::Installed,
                non_empty(evidence.stdout_redacted.trim()),
                evidence,
            ));
        }
        return Ok((CheckStatus::NeedsRepair, None, evidence));
    }

    if step.id == "vercel.install.windows.npm" {
        let evidence = run_program_with_timeout(
            "vercel",
            &["--version"],
            Duration::from_secs(COMMAND_TIMEOUT_SECONDS),
        );
        if evidence.exit_code == Some(0) {
            return Ok((
                CheckStatus::Installed,
                non_empty(evidence.stdout_redacted.trim()),
                evidence,
            ));
        }
        return Ok((CheckStatus::NeedsRepair, None, evidence));
    }

    let verify_step_id = execution_verify_step_id(step.id)
        .ok_or_else(|| format!("검증 레시피가 연결되지 않았습니다: {}", step.id))?;
    let verify_step = find_allowed_command(verify_step_id)
        .ok_or_else(|| format!("검증 명령을 찾을 수 없습니다: {verify_step_id}"))?;

    let evidence = run_allowed_command(&verify_step);
    let (status, detected_version, _, _) = classify_result(&verify_step, &evidence);
    if matches!(status, CheckStatus::Installed) {
        return Ok((status, detected_version, evidence));
    }

    if step.id == "git.install.windows.winget" {
        let fallback_evidence = run_program_with_timeout(
            r"C:\Program Files\Git\cmd\git.exe",
            &["--version"],
            Duration::from_secs(COMMAND_TIMEOUT_SECONDS),
        );
        if fallback_evidence.exit_code == Some(0) {
            return Ok((
                CheckStatus::Installed,
                non_empty(fallback_evidence.stdout_redacted.trim()),
                fallback_evidence,
            ));
        }
    }

    Ok((status, detected_version, evidence))
}

fn install_output_indicates_existing_package(evidence: &CommandEvidence) -> bool {
    let combined = format!(
        "{}\n{}",
        evidence.stdout_redacted.to_ascii_lowercase(),
        evidence.stderr_redacted.to_ascii_lowercase()
    );

    combined.contains("existing package already installed")
        || combined.contains("no newer package versions are available")
        || combined.contains("already installed")
}

fn execute_install_action(
    app: &tauri::AppHandle,
    step: &RecipeStep,
) -> Result<ExecutionOutcome, String> {
    if let Some(target_os) = step.target_os {
        if target_os != std::env::consts::OS {
            return Ok(ExecutionOutcome {
                action_id: step.id.to_string(),
                status: ExecutionStatus::Blocked,
                message_ko: format!(
                    "{} 항목은 현재 운영체제에서 실행할 수 없습니다.",
                    step.label_ko
                ),
                next_action_ko: format!(
                    "이 레시피는 {target_os} 전용입니다. 현재 OS: {}",
                    std::env::consts::OS
                ),
                command_preview: Some(step.command_preview.to_string()),
                docs_url: Some(step.docs_url.to_string()),
            });
        }
    }

    emit_execution_event(
        app,
        SetupExecutionEvent {
            action_id: step.id.to_string(),
            status: ExecutionStatus::NeedsOsConsent,
            kind: "system",
            message_ko: format!(
                "{} 설치를 시작합니다. 권한 창이 뜨면 사용자가 직접 '예'를 눌러주세요.",
                step.label_ko
            ),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        },
    )?;

    let install_evidence = run_program_with_timeout(
        step.program,
        step.args,
        Duration::from_secs(EXECUTION_TIMEOUT_SECONDS),
    );

    if !install_evidence.stdout_redacted.is_empty() {
        emit_execution_event(
            app,
            SetupExecutionEvent {
                action_id: step.id.to_string(),
                status: ExecutionStatus::Running,
                kind: "stdout",
                message_ko: first_non_empty_line(&install_evidence.stdout_redacted)
                    .unwrap_or_else(|| "설치 명령 출력을 받았습니다.".to_string()),
                command_preview: Some(step.command_preview.to_string()),
                docs_url: Some(step.docs_url.to_string()),
            },
        )?;
    }

    if !install_evidence.stderr_redacted.is_empty() {
        emit_execution_event(
            app,
            SetupExecutionEvent {
                action_id: step.id.to_string(),
                status: ExecutionStatus::Running,
                kind: "stderr",
                message_ko: first_non_empty_line(&install_evidence.stderr_redacted)
                    .unwrap_or_else(|| "설치 명령 오류 출력을 받았습니다.".to_string()),
                command_preview: Some(step.command_preview.to_string()),
                docs_url: Some(step.docs_url.to_string()),
            },
        )?;
    }

    let install_finished_or_already_present = install_evidence.exit_code == Some(0)
        || install_output_indicates_existing_package(&install_evidence);

    if !install_finished_or_already_present {
        let detail = first_non_empty_line(&install_evidence.stderr_redacted)
            .or_else(|| first_non_empty_line(&install_evidence.stdout_redacted))
            .unwrap_or_else(|| "설치 명령이 성공 종료 코드를 반환하지 않았습니다.".to_string());
        return Ok(ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::Blocked,
            message_ko: format!("{} 설치가 완료되지 않았습니다.", step.label_ko),
            next_action_ko: format!("설치 로그를 확인하세요: {detail}"),
            command_preview: Some(step.command_preview.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        });
    }

    let verify_step_id = execution_verify_step_id(step.id).unwrap_or(step.id);
    emit_execution_event(
        app,
        SetupExecutionEvent {
            action_id: step.id.to_string(),
            status: ExecutionStatus::Verifying,
            kind: "system",
            message_ko: format!("설치 완료 후 {} 검증을 시작합니다.", verify_step_id),
            command_preview: Some(verify_step_id.to_string()),
            docs_url: Some(step.docs_url.to_string()),
        },
    )?;

    let (verify_status, detected_version, verify_evidence) = verify_after_install(step)?;
    if matches!(verify_status, CheckStatus::Installed) {
        Ok(ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::Done,
            message_ko: format!("{} 설치와 검증이 완료되었습니다.", step.label_ko),
            next_action_ko: detected_version
                .map(|version| format!("검증 결과: {version}. 다음 항목으로 계속 진행합니다."))
                .unwrap_or_else(|| "다음 항목으로 계속 진행합니다.".to_string()),
            command_preview: Some(command_preview_for(step.program, step.args)),
            docs_url: Some(step.docs_url.to_string()),
        })
    } else {
        let detail = first_non_empty_line(&verify_evidence.stderr_redacted)
            .or_else(|| first_non_empty_line(&verify_evidence.stdout_redacted))
            .unwrap_or_else(|| "설치 후 검증 명령이 아직 통과하지 않았습니다.".to_string());
        Ok(ExecutionOutcome {
            action_id: step.id.to_string(),
            status: ExecutionStatus::NeedsReboot,
            message_ko: format!(
                "{} 설치는 끝났지만 현재 앱에서 바로 검증하지 못했습니다.",
                step.label_ko
            ),
            next_action_ko: format!(
                "앱 또는 Windows를 다시 시작한 뒤 재진단하세요. 검증 로그: {detail}"
            ),
            command_preview: Some(command_preview_for(step.program, step.args)),
            docs_url: Some(step.docs_url.to_string()),
        })
    }
}

fn command_label(step: &RecipeStep) -> String {
    if step.args.is_empty() {
        step.program.to_string()
    } else {
        format!("{} {}", step.program, step.args.join(" "))
    }
}

fn run_allowed_command(step: &RecipeStep) -> CommandEvidence {
    run_program_with_timeout(
        step.program,
        step.args,
        Duration::from_secs(COMMAND_TIMEOUT_SECONDS),
    )
}

fn run_program_with_timeout(program: &str, args: &[&str], timeout: Duration) -> CommandEvidence {
    let start = Instant::now();
    let mut command = command_for_program(program, args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    if let Some(path) = refreshed_windows_path() {
        command.env("PATH", path);
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CommandEvidence {
                exit_code: None,
                duration_ms: start.elapsed().as_millis(),
                stdout_redacted: String::new(),
                stderr_redacted: redact(&error.to_string()),
            };
        }
    };

    match child.wait_timeout(timeout).unwrap_or(None) {
        Some(_status) => match child.wait_with_output() {
            Ok(output) => CommandEvidence {
                exit_code: output.status.code(),
                duration_ms: start.elapsed().as_millis(),
                stdout_redacted: redact(&String::from_utf8_lossy(&output.stdout)),
                stderr_redacted: redact(&String::from_utf8_lossy(&output.stderr)),
            },
            Err(error) => CommandEvidence {
                exit_code: None,
                duration_ms: start.elapsed().as_millis(),
                stdout_redacted: String::new(),
                stderr_redacted: redact(&error.to_string()),
            },
        },
        None => {
            let _ = child.kill();
            let _ = child.wait();
            CommandEvidence {
                exit_code: None,
                duration_ms: start.elapsed().as_millis(),
                stdout_redacted: String::new(),
                stderr_redacted: "명령 시간이 초과되어 안전하게 중단했습니다.".to_string(),
            }
        }
    }
}

fn command_for_program(program: &str, args: &[&str]) -> Command {
    #[cfg(target_os = "windows")]
    {
        if matches!(program, "npm" | "pnpm" | "vercel") {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(program).args(args);
            command.creation_flags(CREATE_NO_WINDOW);
            return command;
        }
    }

    let mut command = Command::new(program);
    command.args(args);
    command
}

#[cfg(target_os = "windows")]
fn refreshed_windows_path() -> Option<String> {
    let mut command = Command::new("powershell");
    command
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetEnvironmentVariable('PATH','Machine') + ';' + [Environment]::GetEnvironmentVariable('PATH','User')",
        ])
        .creation_flags(CREATE_NO_WINDOW);
    let output = command.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn classify_result(
    step: &RecipeStep,
    evidence: &CommandEvidence,
) -> (CheckStatus, Option<String>, String, String) {
    let stdout = evidence.stdout_redacted.trim();
    let stderr = evidence.stderr_redacted.trim();

    if evidence.exit_code == Some(0) {
        if step.id == "node.version" && !stdout.starts_with("v24.") {
            return (
                CheckStatus::NeedsRepair,
                non_empty(stdout),
                "Node.js는 있지만 수업 기준인 v24 LTS가 아닙니다.".to_string(),
                "Node.js v24 LTS 설치 전략에 따라 복구하세요.".to_string(),
            );
        }

        return (
            CheckStatus::Installed,
            non_empty(stdout),
            format!("{} 준비되었습니다.", step.label_ko),
            "추가 조치가 필요하지 않습니다.".to_string(),
        );
    }

    if !step.required_for_class && step.id == "wsl.status" {
        return (
            CheckStatus::OptionalSkipped,
            None,
            "WSL은 현재 선택 항목입니다. 필요한 수업에서만 준비하면 됩니다.".to_string(),
            "Linux/WSL 수업이 있으면 강사 안내에 따라 설치하세요.".to_string(),
        );
    }

    if stderr.contains("No such file")
        || stderr.contains("not found")
        || stderr.contains("cannot find")
    {
        return (
            CheckStatus::Missing,
            None,
            format!("{}이(가) 아직 설치되지 않았습니다.", step.label_ko),
            "설치 계획 화면의 안내를 따라 준비하세요.".to_string(),
        );
    }

    (
        CheckStatus::NeedsRepair,
        non_empty(stdout),
        format!("{} 확인이 끝나지 않았습니다.", step.label_ko),
        "권한, 로그인, PATH 또는 네트워크 상태를 확인하세요.".to_string(),
    )
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.lines().next().unwrap_or(value).to_string())
    }
}

fn redact(input: &str) -> String {
    let mut output = input.to_string();
    let replacements = [
        (
            r"(?i)(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{20,}",
            "[REDACTED_TOKEN]",
        ),
        (
            r"(?i)(vercel_[A-Za-z0-9]{20,}|[A-Za-z0-9]{24,}\.[A-Za-z0-9._-]{20,})",
            "[REDACTED_TOKEN]",
        ),
        (
            r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+",
            "$1=[REDACTED_PASSWORD]",
        ),
        (
            r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}",
            "[REDACTED_EMAIL]",
        ),
        (r"/Users/[^/\s]+", "/Users/[REDACTED_USER]"),
        (r"C:\\Users\\[^\\\s]+", "C:\\Users\\[REDACTED_USER]"),
        (
            r"(?i)(device code|code)\s*[:=]\s*[A-Z0-9-]{4,}",
            "$1=[REDACTED_CODE]",
        ),
    ];

    for (pattern, replacement) in replacements {
        let regex = Regex::new(pattern).expect("redaction regex should compile");
        output = regex.replace_all(&output, replacement).to_string();
    }

    output.trim().to_string()
}

fn machine_info() -> MachineInfo {
    MachineInfo {
        os: std::env::consts::OS.to_string(),
        os_version: detect_os_version(),
        build_number: None,
        arch: std::env::consts::ARCH.to_string(),
        shell: std::env::var("SHELL")
            .or_else(|_| std::env::var("ComSpec"))
            .unwrap_or_else(|_| "unknown".to_string()),
        network: NetworkInfo {
            status: "unknown",
            blocked_hosts: vec![],
        },
    }
}

fn detect_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("sw_vers").arg("-productVersion").output();
        if let Ok(output) = output {
            return redact(&String::from_utf8_lossy(&output.stdout));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd").args(["/C", "ver"]).output();
        if let Ok(output) = output {
            return redact(&String::from_utf8_lossy(&output.stdout));
        }
    }

    "unknown".to_string()
}

fn summarize(checks: &[ToolCheck]) -> ReportSummary {
    let required: Vec<&ToolCheck> = checks
        .iter()
        .filter(|check| check.required_for_class)
        .collect();
    let required_total = required.len();
    let required_passed = required
        .iter()
        .filter(|check| matches!(check.status, CheckStatus::Installed))
        .count();
    let has_blocker = checks.iter().any(|check| {
        matches!(
            check.status,
            CheckStatus::Blocked | CheckStatus::Unsupported
        )
    });
    let has_attention = checks.iter().any(|check| {
        matches!(
            check.status,
            CheckStatus::Missing | CheckStatus::NeedsRepair | CheckStatus::NeedsRestart
        ) && check.required_for_class
    });

    let (class_readiness, beginner_message, instructor_message, needs_instructor_help) =
        if has_blocker {
            (
                "blocked",
                "강사 또는 조교의 도움이 필요합니다.".to_string(),
                "차단되었거나 지원 불가인 항목을 먼저 확인하세요.".to_string(),
                true,
            )
        } else if has_attention || required_passed < required_total {
            (
                "needs_attention",
                "몇 가지 항목을 더 준비해야 합니다.".to_string(),
                "복구 필요 또는 설치 필요 항목을 확인하세요.".to_string(),
                true,
            )
        } else {
            (
                "ready_for_class",
                "수업에 필요한 기본 도구가 준비되었습니다.".to_string(),
                "필수 항목 통과, WSL은 선택 항목입니다.".to_string(),
                false,
            )
        };

    ReportSummary {
        class_readiness,
        required_passed,
        required_total,
        needs_instructor_help,
        beginner_message,
        instructor_message,
    }
}

#[tauri::command]
fn get_setup_plan() -> AppPlan {
    AppPlan {
        current_os: std::env::consts::OS,
        steps: allowed_commands()
            .into_iter()
            .map(|step| RecipeStepView {
                verify_command_label: command_label(&step),
                id: step.id,
                target_os: step.target_os,
                label_ko: step.label_ko,
                description_ko: step.description_ko,
                required_for_class: step.required_for_class,
                requires_consent: step.requires_consent,
                may_require_elevation: step.may_require_elevation,
                requires_browser: step.requires_browser,
                required_version_hint: step.required_version_hint,
                docs_url: step.docs_url,
                risk_tier: step.risk_tier,
                action_phase: step.action_phase,
                approval_copy_ko: step.approval_copy_ko,
                expected_permission_prompt_ko: step.expected_permission_prompt_ko,
                package_source: step.package_source,
                rollback_note_ko: step.rollback_note_ko,
                support_handoff_ko: step.support_handoff_ko,
                command_preview: step.command_preview,
                requires_elevation_method: step.requires_elevation_method,
            })
            .collect(),
        forbidden_commands: vec![
            "vercel login --github",
            "vercel login --gitlab",
            "vercel login --bitbucket",
            "vercel login --oob",
            "--token",
            "sh -c",
            "cmd /C <user-input>",
        ],
        security_notes: vec![
            "이 프로그램은 비밀번호를 묻지 않습니다.",
            "GitHub와 Vercel 로그인은 공식 브라우저 흐름만 사용합니다.",
            "모든 명령은 allowlist와 structured args로만 실행합니다.",
            "리포트 export 전 민감정보를 가립니다.",
        ],
    }
}

#[tauri::command]
fn run_diagnostic(command_id: String) -> Result<RunResult, String> {
    let step = find_allowed_command(&command_id)
        .ok_or_else(|| format!("허용되지 않은 명령 ID입니다: {command_id}"))?;

    if step.requires_consent {
        return Err("이 명령은 브라우저 로그인 또는 사용자 동의가 필요합니다. UI 안내 버튼에서 별도 동의를 받은 뒤 실행해야 합니다.".to_string());
    }

    let evidence = run_allowed_command(&step);
    let (status, detected_version, beginner_message, support_action) =
        classify_result(&step, &evidence);

    Ok(RunResult {
        command_id: step.id.to_string(),
        command_label: command_label(&step),
        status,
        detected_version,
        beginner_message,
        support_action,
        evidence,
    })
}

#[tauri::command]
fn execute_setup_action(
    app: tauri::AppHandle,
    input: ExecuteSetupActionInput,
) -> Result<ExecutionOutcome, String> {
    if input.approval_id.trim().is_empty() {
        return Err("승인 ID가 비어 있어 작업을 시작할 수 없습니다.".to_string());
    }

    let step = find_allowed_command(&input.action_id)
        .ok_or_else(|| format!("허용되지 않은 작업 ID입니다: {}", input.action_id))?;

    let started = SetupExecutionEvent {
        action_id: step.id.to_string(),
        status: ExecutionStatus::Running,
        kind: "system",
        message_ko: format!("{} 작업을 allowlist에서 확인했습니다.", step.label_ko),
        command_preview: Some(step.command_preview.to_string()),
        docs_url: Some(step.docs_url.to_string()),
    };
    emit_execution_event(&app, started)?;

    if step.action_phase == ActionPhase::Install {
        let outcome = execute_install_action(&app, &step)?;
        let kind = if matches!(
            outcome.status,
            ExecutionStatus::Blocked | ExecutionStatus::NeedsReboot
        ) {
            "stderr"
        } else {
            "system"
        };
        emit_execution_event(&app, execution_event_from_outcome(&outcome, kind))?;
        return Ok(outcome);
    }

    if step.action_phase == ActionPhase::ExternalFlow {
        let outcome = execute_external_flow_action(&step)?;
        let kind = if outcome.status == ExecutionStatus::Blocked {
            "stderr"
        } else {
            "system"
        };
        emit_execution_event(&app, execution_event_from_outcome(&outcome, kind))?;
        return Ok(outcome);
    }

    let outcome = execution_outcome_for(&step);
    let kind = if outcome.status == ExecutionStatus::Blocked {
        "stderr"
    } else {
        "system"
    };
    emit_execution_event(&app, execution_event_from_outcome(&outcome, kind))?;

    Ok(outcome)
}

#[tauri::command]
fn run_all_diagnostics() -> Vec<ToolCheck> {
    allowed_commands()
        .into_iter()
        .filter(|step| !step.requires_consent)
        .filter(|step| {
            step.required_for_class
                || (cfg!(target_os = "macos") && step.id.starts_with("macos"))
                || (cfg!(target_os = "macos") && step.id == "brew.version")
                || (cfg!(target_os = "windows") && step.id.starts_with("windows"))
                || step.id == "wsl.status"
        })
        .map(|step| {
            let evidence = run_allowed_command(&step);
            let (status, detected_version, beginner_message, support_action) =
                classify_result(&step, &evidence);
            ToolCheck {
                id: step.id.to_string(),
                label: step.label_ko.to_string(),
                required_for_class: step.required_for_class,
                status,
                detected_version,
                required_version: step.required_version_hint.map(ToString::to_string),
                verify_command_label: command_label(&step),
                beginner_message,
                support_action,
                evidence,
                links: vec![step.docs_url.to_string()],
            }
        })
        .collect()
}

#[tauri::command]
fn build_health_report(input: HealthReportInput) -> HealthReport {
    let checks = input.checks;
    let summary = summarize(&checks);
    let execution_log = checks
        .iter()
        .map(|check| LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            step_id: check.id.clone(),
            command_label: check.verify_command_label.clone(),
            status: if matches!(
                check.status,
                CheckStatus::Installed | CheckStatus::OptionalSkipped
            ) {
                "passed"
            } else {
                "failed"
            },
            exit_code: check.evidence.exit_code,
            duration_ms: check.evidence.duration_ms,
            message_redacted: check.beginner_message.clone(),
        })
        .collect();

    HealthReport {
        schema_version: "0.1.0",
        generated_at: Utc::now().to_rfc3339(),
        app: AppInfo {
            name: "위드지니 셋업",
            version: APP_VERSION,
            recipe_version: RECIPE_VERSION,
            distribution_channel: "public_github",
        },
        machine: machine_info(),
        summary,
        checks,
        execution_log,
        resume_state: ResumeState {
            last_completed_step: "diagnostics",
            next_action: "follow_support_action_or_start_lesson",
            requires_restart: false,
        },
        redaction: RedactionInfo {
            applied: true,
            rules_version: "0.1.0",
            masked_fields: vec![
                "token_like_strings",
                "password_like_values",
                "email_addresses",
                "home_paths",
                "oauth_device_codes",
            ],
        },
    }
}

#[tauri::command]
fn build_handoff_packet(input: HandoffPacketInput) -> HandoffPacket {
    let summary = summarize(&input.checks);
    let help_requested = input
        .approval_cards
        .iter()
        .any(|card| card.decision == "ask_instructor");
    let failed_count = input
        .checks
        .iter()
        .filter(|check| {
            !matches!(
                check.status,
                CheckStatus::Installed | CheckStatus::OptionalSkipped
            )
        })
        .count();

    HandoffPacket {
        generated_at: Utc::now().to_rfc3339(),
        student_summary_ko: summary.beginner_message,
        instructor_summary_ko: summary.instructor_message,
        next_action_ko: if help_requested {
            "승인 큐에서 강사 도움 요청으로 표시된 항목을 먼저 확인하세요.".to_string()
        } else if failed_count > 0 {
            "실패한 항목의 support_action과 redacted evidence를 확인하세요.".to_string()
        } else {
            "필수 항목이 준비되었습니다.".to_string()
        },
        checks: input.checks,
        approval_cards: input.approval_cards,
        redaction: RedactionInfo {
            applied: true,
            rules_version: "0.1.0",
            masked_fields: vec![
                "token_like_strings",
                "password_like_values",
                "email_addresses",
                "home_paths",
                "oauth_device_codes",
            ],
        },
    }
}

#[tauri::command]
fn preview_redaction(sample: String) -> String {
    redact(&sample)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            install_korean_native_menu(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_setup_plan,
            run_diagnostic,
            execute_setup_action,
            run_all_diagnostics,
            build_health_report,
            build_handoff_packet,
            preview_redaction
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn install_korean_native_menu<R: tauri::Runtime>(app: &mut tauri::App<R>) -> tauri::Result<()> {
    let about = AboutMetadata {
        name: Some("위드지니 셋업".to_string()),
        version: Some(APP_VERSION.to_string()),
        copyright: Some("© WithGenie".to_string()),
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    let app_menu = SubmenuBuilder::new(app, "위드지니 셋업")
        .about_with_text("위드지니 셋업 정보", Some(about))
        .separator()
        .services_with_text("서비스")
        .separator()
        .hide_with_text("위드지니 셋업 가리기")
        .hide_others_with_text("다른 앱 가리기")
        .show_all_with_text("모두 보이기")
        .separator()
        .quit_with_text("위드지니 셋업 종료")
        .build()?;

    #[cfg(not(target_os = "macos"))]
    let app_menu = SubmenuBuilder::new(app, "위드지니 셋업")
        .about_with_text("위드지니 셋업 정보", Some(about))
        .separator()
        .quit_with_text("위드지니 셋업 종료")
        .build()?;

    let file_menu = SubmenuBuilder::new(app, NATIVE_MENU_LABELS_KO[0])
        .close_window_with_text("창 닫기")
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, NATIVE_MENU_LABELS_KO[1])
        .undo_with_text("실행 취소")
        .redo_with_text("다시 실행")
        .separator()
        .cut_with_text("잘라내기")
        .copy_with_text("복사")
        .paste_with_text("붙여넣기")
        .select_all_with_text("전체 선택")
        .build()?;
    let view_menu = SubmenuBuilder::new(app, NATIVE_MENU_LABELS_KO[2])
        .fullscreen_with_text("전체 화면 전환")
        .build()?;
    let window_menu = SubmenuBuilder::new(app, NATIVE_MENU_LABELS_KO[3])
        .minimize_with_text("최소화")
        .maximize_with_text("확대/축소")
        .separator()
        .bring_all_to_front_with_text("모든 창 앞으로")
        .build()?;
    let help_menu = SubmenuBuilder::new(app, NATIVE_MENU_LABELS_KO[4])
        .text("security-help", "보안 안내: 비밀번호를 묻지 않습니다")
        .text("report-help", "리포트는 민감정보를 가립니다")
        .build()?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ])
        .build()?;

    app.set_menu(menu)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_command_is_not_allowed() {
        assert!(find_allowed_command("vercel.login.--github").is_none());
        assert!(find_allowed_command("shell.raw").is_none());
    }

    #[test]
    fn vercel_login_has_no_deprecated_provider_flags() {
        let step = find_allowed_command("vercel.login").expect("vercel login should exist");
        assert_eq!(step.args, &["login"]);
        assert!(!step.args.iter().any(|arg| matches!(
            *arg,
            "--github" | "--gitlab" | "--bitbucket" | "--oob" | "--token"
        )));
    }

    #[test]
    fn github_login_uses_browser_clipboard_flow_without_prompting_for_tokens() {
        let step = find_allowed_command("gh.auth.login").expect("GitHub login should exist");

        assert_eq!(step.program, "gh");
        assert!(step.args.contains(&"--web"));
        assert!(step.args.contains(&"--clipboard"));
        assert!(step.args.contains(&"--hostname"));
        assert!(step.args.contains(&"github.com"));
        assert!(step.args.contains(&"--git-protocol"));
        assert!(step.args.contains(&"https"));
        assert!(!step.args.contains(&"--with-token"));
    }

    #[test]
    fn external_login_flows_have_launcher_commands_and_verify_steps() {
        let github = external_flow_command_for("gh.auth.login")
            .expect("GitHub external flow should have a launcher command");
        assert_eq!(github.program, "gh");
        assert_eq!(github.verify_program, "gh");
        assert_eq!(github.verify_args, ["auth", "status"]);

        let vercel = external_flow_command_for("vercel.login")
            .expect("Vercel external flow should have a launcher command");
        assert_eq!(vercel.program, "vercel");
        assert_eq!(vercel.args, ["login"]);
        assert_eq!(vercel.verify_program, "vercel");
        assert_eq!(vercel.verify_args, ["whoami"]);
    }

    #[test]
    fn redacts_sensitive_values() {
        let sample = "token=ghp_abcdefghijklmnopqrstuvwxyz user=a@example.com path=/Users/genie code: ABCD-1234";
        let redacted = redact(sample);
        assert!(redacted.contains("[REDACTED_TOKEN]"));
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("/Users/[REDACTED_USER]"));
        assert!(redacted.contains("[REDACTED_CODE]"));
        assert!(!redacted.contains("a@example.com"));
    }

    #[test]
    fn health_report_marks_redaction_applied() {
        let report = build_health_report(HealthReportInput { checks: vec![] });
        assert!(report.redaction.applied);
        assert_eq!(report.schema_version, "0.1.0");
    }

    #[test]
    fn consent_recipes_are_user_mediated_external_flows() {
        let gh = find_allowed_command("gh.auth.login").expect("GitHub login recipe should exist");
        let vercel =
            find_allowed_command("vercel.login").expect("Vercel login recipe should exist");

        assert_eq!(gh.risk_tier, RiskTier::UserMediated);
        assert_eq!(gh.action_phase, ActionPhase::ExternalFlow);
        assert_eq!(vercel.risk_tier, RiskTier::UserMediated);
        assert_eq!(vercel.action_phase, ActionPhase::ExternalFlow);
    }

    #[test]
    fn diagnostic_recipes_are_safe_detect_actions() {
        let node = find_allowed_command("node.version").expect("Node version recipe should exist");
        assert_eq!(node.risk_tier, RiskTier::Safe);
        assert_eq!(node.action_phase, ActionPhase::Detect);
        assert!(!node.requires_consent);
    }

    #[test]
    fn windows_git_install_recipe_requires_explicit_os_consent() {
        let git_install = find_allowed_command("git.install.windows.winget")
            .expect("Windows Git install recipe should exist");

        assert_eq!(git_install.target_os, Some("windows"));
        assert_eq!(git_install.risk_tier, RiskTier::PermissionPrompt);
        assert_eq!(git_install.action_phase, ActionPhase::Install);
        assert!(git_install.requires_consent);
        assert!(git_install.may_require_elevation);
        assert_eq!(
            execution_verify_step_id(git_install.id),
            Some("git.version")
        );
        assert!(git_install.command_preview.contains("winget install"));
        assert!(git_install.expected_permission_prompt_ko.contains("UAC"));
    }

    #[test]
    fn winget_existing_package_output_allows_verification_instead_of_blocking() {
        let evidence = CommandEvidence {
            exit_code: Some(1),
            duration_ms: 42,
            stdout_redacted: "Found an existing package already installed. Trying to upgrade the installed package...".to_string(),
            stderr_redacted: String::new(),
        };

        assert!(install_output_indicates_existing_package(&evidence));
    }

    #[test]
    fn windows_one_click_install_recipes_have_safe_verification_targets() {
        let node_install = find_allowed_command("node.install.windows.winget")
            .expect("Windows Node install recipe should exist");
        let pnpm_install = find_allowed_command("pnpm.install.windows.npm")
            .expect("Windows pnpm install recipe should exist");
        let gh_install = find_allowed_command("gh.install.windows.winget")
            .expect("Windows GitHub CLI install recipe should exist");
        let vercel_install = find_allowed_command("vercel.install.windows.npm")
            .expect("Windows Vercel CLI install recipe should exist");

        assert_eq!(
            execution_verify_step_id(node_install.id),
            Some("node.version")
        );
        assert_eq!(
            execution_verify_step_id(pnpm_install.id),
            Some("pnpm.version")
        );
        assert!(node_install.command_preview.contains("OpenJS.NodeJS.LTS"));
        assert!(gh_install.command_preview.contains("GitHub.cli"));
        assert!(vercel_install.command_preview.contains("vercel@latest"));
        assert_eq!(
            pnpm_install.requires_elevation_method,
            ElevationMethod::None
        );
        assert_eq!(
            vercel_install.requires_elevation_method,
            ElevationMethod::None
        );
    }

    #[test]
    fn detect_recipe_execution_is_blocked_until_install_recipe_exists() {
        let node = find_allowed_command("node.version").expect("Node version recipe should exist");
        let outcome = execution_outcome_for(&node);

        assert_eq!(outcome.status, ExecutionStatus::Blocked);
        assert!(outcome
            .next_action_ko
            .contains("설치/복구 레시피가 연결되기 전"));
    }

    #[test]
    fn external_flow_execution_requires_browser_auth() {
        let vercel =
            find_allowed_command("vercel.login").expect("Vercel login recipe should exist");
        let outcome = execution_outcome_for(&vercel);

        assert_eq!(outcome.status, ExecutionStatus::NeedsBrowserAuth);
        assert_eq!(outcome.command_preview.as_deref(), Some("vercel login"));
    }

    #[test]
    fn handoff_packet_preserves_redaction_contract() {
        let packet = build_handoff_packet(HandoffPacketInput {
            checks: vec![],
            approval_cards: vec![ApprovalCardInput {
                id: "node.version".to_string(),
                label: "Node.js 버전 확인".to_string(),
                decision: "ask_instructor".to_string(),
                reason_ko: "설치가 필요합니다.".to_string(),
            }],
        });

        assert!(packet.redaction.applied);
        assert!(packet.next_action_ko.contains("강사 도움 요청"));
    }

    #[test]
    fn native_menu_labels_are_korean() {
        assert_eq!(
            NATIVE_MENU_LABELS_KO,
            ["파일", "편집", "보기", "창", "도움말"]
        );
        assert!(!NATIVE_MENU_LABELS_KO
            .iter()
            .any(|label| matches!(*label, "File" | "Edit" | "View" | "Window" | "Help")));
    }
}
