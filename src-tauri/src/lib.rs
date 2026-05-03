use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::menu::{AboutMetadata, MenuBuilder, SubmenuBuilder};
use wait_timeout::ChildExt;

const APP_VERSION: &str = "0.1.0";
const RECIPE_VERSION: &str = "2026.05.01";
const COMMAND_TIMEOUT_SECONDS: u64 = 12;
const NATIVE_MENU_LABELS_KO: [&str; 5] = ["파일", "편집", "보기", "창", "도움말"];

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
    steps: Vec<RecipeStepView>,
    forbidden_commands: Vec<&'static str>,
    security_notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct RecipeStepView {
    id: &'static str,
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

fn allowed_commands() -> Vec<RecipeStep> {
    vec![
        RecipeStep {
            id: "node.version",
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
            id: "npm.version",
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
            id: "git.version",
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
            id: "gh.auth.status",
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
            id: "gh.auth.login",
            label_ko: "GitHub 브라우저 로그인 시작",
            description_ko: "GitHub 로그인은 브라우저에서 진행됩니다. 이 앱은 GitHub 비밀번호를 묻지 않습니다.",
            program: "gh",
            args: &["auth", "login"],
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
            command_preview: "gh auth login",
            requires_elevation_method: ElevationMethod::UserManaged,
        },
        RecipeStep {
            id: "vercel.whoami",
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
            id: "vercel.login",
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

fn command_label(step: &RecipeStep) -> String {
    if step.args.is_empty() {
        step.program.to_string()
    } else {
        format!("{} {}", step.program, step.args.join(" "))
    }
}

fn run_allowed_command(step: &RecipeStep) -> CommandEvidence {
    let start = Instant::now();
    let mut child = match Command::new(step.program)
        .args(step.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
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

    match child
        .wait_timeout(Duration::from_secs(COMMAND_TIMEOUT_SECONDS))
        .unwrap_or(None)
    {
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
        steps: allowed_commands()
            .into_iter()
            .map(|step| RecipeStepView {
                verify_command_label: command_label(&step),
                id: step.id,
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
        .filter(|check| !matches!(check.status, CheckStatus::Installed | CheckStatus::OptionalSkipped))
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
        let vercel = find_allowed_command("vercel.login").expect("Vercel login recipe should exist");

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
