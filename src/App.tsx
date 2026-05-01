import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import "./App.css";

type CheckStatus =
  | "installed"
  | "missing"
  | "needs_repair"
  | "needs_restart"
  | "optional_skipped"
  | "unsupported"
  | "blocked";

type RecipeStep = {
  id: string;
  label_ko: string;
  description_ko: string;
  verify_command_label: string;
  required_for_class: boolean;
  requires_consent: boolean;
  may_require_elevation: boolean;
  requires_browser: boolean;
  required_version_hint?: string | null;
  docs_url: string;
};

type SetupPlan = {
  steps: RecipeStep[];
  forbidden_commands: string[];
  security_notes: string[];
};

type CommandEvidence = {
  exit_code?: number | null;
  duration_ms: number;
  stdout_redacted: string;
  stderr_redacted: string;
};

type ToolCheck = {
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

type HealthReport = {
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

type ScreenId = "overview" | "plan" | "diagnostics" | "report" | "help";
type BusyTask = "plan" | "diagnostics" | "report" | "update" | null;
type StepState = "done" | "current" | "waiting";

const statusLabels: Record<CheckStatus, string> = {
  installed: "준비됨",
  missing: "설치 필요",
  needs_repair: "복구 필요",
  needs_restart: "재시작 필요",
  optional_skipped: "선택 항목 건너뜀",
  unsupported: "지원 불가",
  blocked: "차단됨",
};

const readinessLabels: Record<HealthReport["summary"]["class_readiness"], string> = {
  ready_for_class: "수업 가능",
  needs_attention: "확인 필요",
  blocked: "강사 지원 필요",
  unsupported: "지원 불가",
};

const actionStatuses: CheckStatus[] = ["missing", "needs_repair", "needs_restart", "blocked", "unsupported"];

function App() {
  const [activeScreen, setActiveScreen] = useState<ScreenId>("overview");
  const [plan, setPlan] = useState<SetupPlan | null>(null);
  const [checks, setChecks] = useState<ToolCheck[]>([]);
  const [report, setReport] = useState<HealthReport | null>(null);
  const [busyTask, setBusyTask] = useState<BusyTask>(null);
  const [message, setMessage] = useState("앱을 시작하면 현재 컴퓨터 상태를 안전하게 진단합니다.");

  const isBusy = busyTask !== null;
  const requiredCount = useMemo(
    () => checks.filter((check) => check.required_for_class).length,
    [checks],
  );

  const requiredPassed = useMemo(
    () => checks.filter((check) => check.required_for_class && check.status === "installed").length,
    [checks],
  );

  const needsActionCount = useMemo(
    () => checks.filter((check) => actionStatuses.includes(check.status)).length,
    [checks],
  );

  const currentReadiness = report ? readinessLabels[report.summary.class_readiness] : checks.length > 0 ? "리포트 준비됨" : "진단 전";
  const progressPercent = report
    ? Math.round((report.summary.required_passed / Math.max(report.summary.required_total, 1)) * 100)
    : checks.length > 0
      ? Math.round((requiredPassed / Math.max(requiredCount, 1)) * 100)
      : plan
        ? 35
        : 12;

  const navItems = useMemo(
    () => [
      { id: "overview" as ScreenId, label: "시작", helper: "안전 안내 확인", state: (activeScreen === "overview" ? "current" : "done") as StepState },
      {
        id: "plan" as ScreenId,
        label: "설치 계획",
        helper: plan ? `${plan.steps.length}개 항목 준비` : "아직 안 봄",
        state: (activeScreen === "plan" ? "current" : plan ? "done" : "waiting") as StepState,
      },
      {
        id: "diagnostics" as ScreenId,
        label: "진단 결과",
        helper: busyTask === "diagnostics" ? "진행 중" : checks.length ? `${checks.length}개 점검` : "대기",
        state: (busyTask === "diagnostics" || activeScreen === "diagnostics" ? "current" : checks.length ? "done" : "waiting") as StepState,
      },
      {
        id: "report" as ScreenId,
        label: "리포트",
        helper: report ? "복사 가능" : checks.length ? "생성 가능" : "진단 후 가능",
        state: (activeScreen === "report" ? "current" : report ? "done" : "waiting") as StepState,
      },
      { id: "help" as ScreenId, label: "도움말", helper: needsActionCount ? "막힐 때" : "참고", state: (activeScreen === "help" ? "current" : "waiting") as StepState },
    ],
    [activeScreen, busyTask, checks.length, needsActionCount, plan, report],
  );

  async function loadPlan() {
    setBusyTask("plan");
    setActiveScreen("plan");
    setMessage("설치 계획을 불러오는 중입니다...");
    try {
      const setupPlan = await invoke<SetupPlan>("get_setup_plan");
      setPlan(setupPlan);
      setMessage("설치 계획을 불러왔습니다. 아직 실제 설치 명령은 실행하지 않았습니다.");
    } catch (error) {
      setMessage(`설치 계획을 불러오지 못했습니다: ${String(error)}`);
    } finally {
      setBusyTask(null);
    }
  }

  async function runDiagnostics() {
    setBusyTask("diagnostics");
    setActiveScreen("diagnostics");
    setMessage("진단 중입니다. 허용된 확인 명령만 실행합니다...");
    try {
      const nextChecks = await invoke<ToolCheck[]>("run_all_diagnostics");
      setChecks(nextChecks);
      const nextReport = await invoke<HealthReport>("build_health_report", {
        input: { checks: nextChecks },
      });
      setReport(nextReport);
      setMessage(nextReport.summary.beginner_message);
    } catch (error) {
      setMessage(`진단 중 문제가 생겼습니다: ${String(error)}`);
    } finally {
      setBusyTask(null);
    }
  }

  async function buildReport() {
    setBusyTask("report");
    setActiveScreen("report");
    setMessage("리포트를 새로 만드는 중입니다...");
    try {
      const nextReport = await invoke<HealthReport>("build_health_report", {
        input: { checks },
      });
      setReport(nextReport);
      setMessage("리포트를 새로 만들었습니다. 민감정보 가림 처리가 적용되었습니다.");
    } catch (error) {
      setMessage(`리포트를 만들지 못했습니다: ${String(error)}`);
    } finally {
      setBusyTask(null);
    }
  }

  async function copyReport() {
    try {
      const nextReport = report ?? (await invoke<HealthReport>("build_health_report", { input: { checks } }));
      setReport(nextReport);
      await navigator.clipboard.writeText(JSON.stringify(nextReport, null, 2));
      setMessage("리포트 내용을 클립보드에 복사했습니다.");
    } catch (error) {
      setMessage(`리포트를 복사하지 못했습니다: ${String(error)}`);
    }
  }

  async function checkForUpdates() {
    setBusyTask("update");
    setActiveScreen("help");
    setMessage("GitHub 릴리즈에서 새 버전을 확인하는 중입니다...");
    try {
      const update = await check({ timeout: 8000 });
      if (!update) {
        setMessage("현재 설치된 위드지니 셋업이 최신 버전입니다.");
        return;
      }

      const shouldInstall = window.confirm(
        `새 버전 ${update.version}이 있습니다.\n\n지금 내려받고 설치할까요? 앱이 다시 시작될 수 있습니다.`,
      );

      if (!shouldInstall) {
        setMessage(`새 버전 ${update.version} 설치를 나중으로 미뤘습니다.`);
        return;
      }

      setMessage(`새 버전 ${update.version}을 내려받고 설치하는 중입니다...`);
      await update.downloadAndInstall();
      setMessage("업데이트 설치가 끝났습니다. 앱을 다시 시작합니다...");
      await relaunch();
    } catch (error) {
      setMessage(`업데이트를 확인하지 못했습니다: ${String(error)}`);
    } finally {
      setBusyTask(null);
    }
  }

  return (
    <main className="app-shell">
      <header className="top-bar">
        <div className="brand-block">
          <p className="eyebrow">위드지니 개발 환경 도우미</p>
          <h1>위드지니 셋업</h1>
          <p>수업 전 개발 환경을 안전하게 점검합니다.</p>
        </div>
        <div className="top-actions" aria-label="주요 작업">
          <button type="button" onClick={loadPlan} disabled={isBusy}>설치 계획</button>
          <button type="button" className="primary" onClick={runDiagnostics} disabled={isBusy}>
            {busyTask === "diagnostics" ? "진단 중" : checks.length ? "다시 진단" : "안전 진단 시작"}
          </button>
          <button type="button" onClick={copyReport} disabled={checks.length === 0 || isBusy}>리포트 복사</button>
        </div>
      </header>

      <section className="status-strip" role="status" aria-live="polite">
        <div>
          <span className="status-label">현재 상태</span>
          <strong>{message}</strong>
        </div>
        <div className="status-meters" aria-label="진행 요약">
          <span>준비도 {progressPercent}%</span>
          <span>필수 {requiredPassed}/{requiredCount || "-"}</span>
          <span>{currentReadiness}</span>
        </div>
      </section>

      <div className="workspace">
        <aside className="side-rail" aria-label="진행 상태와 화면 이동">
          <nav className="flow-card" aria-label="진행 상태와 화면 이동">
            <div className="rail-title">
              <strong>진행 상태</strong>
              <span>항상 여기서 확인</span>
            </div>
            {navItems.map((item, index) => (
              <button
                type="button"
                className={`rail-step ${item.state} ${activeScreen === item.id ? "active" : ""}`}
                key={item.id}
                onClick={() => setActiveScreen(item.id)}
                aria-current={activeScreen === item.id ? "page" : undefined}
              >
                <span>{index + 1}</span>
                <div>
                  <strong>{item.label}</strong>
                  <small>{item.helper}</small>
                </div>
              </button>
            ))}
          </nav>

          <div className="summary-card">
            <span className={`readiness ${report?.summary.class_readiness ?? "pending"}`}>{currentReadiness}</span>
            <p>{report?.summary.beginner_message ?? "진단을 시작하면 수업 준비 여부가 여기에 고정 표시됩니다."}</p>
            <div className="progress-bar" aria-label={`준비도 ${progressPercent}%`}>
              <span style={{ width: `${progressPercent}%` }} />
            </div>
          </div>
        </aside>

        <section className="screen-card" aria-label="선택한 화면">
          {activeScreen === "overview" && renderOverview()}
          {activeScreen === "plan" && renderPlan(plan, loadPlan, isBusy)}
          {activeScreen === "diagnostics" && renderDiagnostics(checks, buildReport, isBusy)}
          {activeScreen === "report" && renderReport(report, checks, buildReport, copyReport, isBusy)}
          {activeScreen === "help" && renderHelp(plan, checkForUpdates, isBusy)}
        </section>
      </div>

      <footer>
        <span>문서 기준 초기 버전 · 실제 설치 명령은 사용자 동의와 승인 절차 후 단계적으로 추가합니다.</span>
        <span>필수 항목 수: {requiredCount || "진단 전"}</span>
      </footer>

      {busyTask && <ProgressModal task={busyTask} />}
    </main>
  );
}

function renderOverview() {
  return (
    <div className="screen-stack overview-stack simple-overview">
      <section className="screen-hero compact minimal-hero">
        <div>
          <p className="eyebrow">시작</p>
          <h2>수업 준비 상태를 간단히 확인합니다.</h2>
          <p>필요한 도구만 확인하고, 문제는 리포트로 공유합니다.</p>
        </div>
      </section>

      <section className="quick-grid minimal-grid">
        <article className="info-card">
          <span className="card-kicker">1</span>
          <strong>설치 계획</strong>
          <p>필수 항목만 먼저 확인합니다.</p>
        </article>
        <article className="info-card">
          <span className="card-kicker">2</span>
          <strong>안전 진단</strong>
          <p>비밀번호 없이 현재 상태를 읽습니다.</p>
        </article>
        <article className="info-card">
          <span className="card-kicker">3</span>
          <strong>리포트</strong>
          <p>막히면 강사에게 바로 보냅니다.</p>
        </article>
      </section>

      <section className="safety-strip" aria-label="안전 원칙">
        <span>비밀번호 수집 안 함</span>
        <span>확인 명령만 실행</span>
        <span>민감정보 가림</span>
      </section>
    </div>
  );
}

function renderPlan(plan: SetupPlan | null, loadPlan: () => void, isBusy: boolean) {
  return (
    <div className="screen-stack plan-stack">
      <div className="screen-heading">
        <div>
          <p className="eyebrow">설치 계획</p>
          <h2>수업 전에 무엇을 확인하는지 한눈에 봅니다.</h2>
        </div>
        <button type="button" onClick={loadPlan} disabled={isBusy}>계획 새로 불러오기</button>
      </div>

      {!plan ? (
        <EmptyState
          title="아직 설치 계획을 불러오지 않았습니다."
          body="설치 계획 보기를 누르면 필수 항목, 브라우저 로그인 여부, 권한 필요 여부를 먼저 확인합니다."
          actionLabel="설치 계획 보기"
          onAction={loadPlan}
          disabled={isBusy}
        />
      ) : (
        <div className="content-scroll plan-scroll">
          {plan.steps.map((step) => (
            <article className="step-item" key={step.id}>
              <div>
                <strong>{step.label_ko}</strong>
                <p>{step.description_ko}</p>
                <code>{step.verify_command_label}</code>
              </div>
              <div className="badges">
                {step.required_for_class ? <span className="badge required">필수</span> : <span className="badge optional">선택</span>}
                {step.requires_browser && <span className="badge">브라우저 로그인</span>}
                {step.may_require_elevation && <span className="badge warning">권한 필요</span>}
                {step.required_version_hint && <span className="badge">{step.required_version_hint}</span>}
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

function renderDiagnostics(checks: ToolCheck[], buildReport: () => void, isBusy: boolean) {
  return (
    <div className="screen-stack diagnostics-stack">
      <div className="screen-heading">
        <div>
          <p className="eyebrow">진단 결과</p>
          <h2>수업 준비 상태를 항목별로 확인합니다.</h2>
        </div>
        <button type="button" onClick={buildReport} disabled={checks.length === 0 || isBusy}>리포트 새로 만들기</button>
      </div>

      {checks.length === 0 ? (
        <EmptyState
          title="아직 진단 결과가 없습니다."
          body="안전 진단 시작을 누르면 허용된 확인 명령만 실행하고, 결과 화면에 바로 표시합니다."
        />
      ) : (
        <div className="content-scroll result-grid compact-results">
          {checks.map((check) => (
            <article className={`result-card ${check.status}`} key={check.id}>
              <div className="result-title">
                <strong>{check.label}</strong>
                <span>{statusLabels[check.status]}</span>
              </div>
              <p>{check.beginner_message}</p>
              <dl>
                <dt>검증</dt>
                <dd><code>{check.verify_command_label}</code></dd>
                <dt>감지 값</dt>
                <dd>{check.detected_version ?? "확인 안 됨"}</dd>
                <dt>다음 조치</dt>
                <dd>{check.support_action}</dd>
              </dl>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

function renderReport(
  report: HealthReport | null,
  checks: ToolCheck[],
  buildReport: () => void,
  copyReport: () => void,
  isBusy: boolean,
) {
  return (
    <div className="screen-stack report-stack">
      <div className="screen-heading">
        <div>
          <p className="eyebrow">리포트</p>
          <h2>강사에게 보낼 요약을 확인합니다.</h2>
        </div>
        <div className="inline-actions">
          <button type="button" onClick={buildReport} disabled={checks.length === 0 || isBusy}>새로 만들기</button>
          <button type="button" className="primary" onClick={copyReport} disabled={checks.length === 0 || isBusy}>복사하기</button>
        </div>
      </div>

      {!report ? (
        <EmptyState
          title="진단 후 리포트가 만들어집니다."
          body="리포트에는 민감정보 가림 처리가 적용됩니다. 문제가 생기면 이 내용을 강사 또는 조교에게 전달하세요."
        />
      ) : (
        <div className="report-layout">
          <article className="report-summary">
            <span className={`readiness ${report.summary.class_readiness}`}>{readinessLabels[report.summary.class_readiness]}</span>
            <h3>{report.summary.beginner_message}</h3>
            <p>{report.summary.instructor_message}</p>
          </article>
          <div className="report-facts">
            <div><strong>{report.summary.required_passed}/{report.summary.required_total}</strong><span>필수 항목 통과</span></div>
            <div><strong>{report.checks.length}</strong><span>전체 점검 항목</span></div>
            <div><strong>{report.redaction.applied ? "적용됨" : "확인 필요"}</strong><span>민감정보 가림</span></div>
            <div><strong>{report.summary.needs_instructor_help ? "필요" : "불필요"}</strong><span>강사 지원</span></div>
          </div>
        </div>
      )}
    </div>
  );
}

function renderHelp(plan: SetupPlan | null, checkForUpdates: () => void, isBusy: boolean) {
  return (
    <div className="screen-stack help-stack">
      <div className="screen-heading">
        <div>
          <p className="eyebrow">도움말</p>
          <h2>막히면 이렇게 진행하세요.</h2>
        </div>
      </div>
      <div className="support-list">
        <div><strong>수업 가능</strong><p>리포트를 저장하고 수업 프로젝트를 열면 됩니다.</p></div>
        <div><strong>복구 필요</strong><p>표시된 항목의 복구 안내를 따라 새 터미널에서 다시 확인하세요.</p></div>
        <div><strong>강사 지원 필요</strong><p>리포트 내용을 강사 또는 조교에게 전달하세요.</p></div>
        <div><strong>업데이트</strong><p>GitHub 릴리즈에서 새 버전이 있는지 확인합니다.</p><button type="button" onClick={checkForUpdates} disabled={isBusy}>업데이트 확인</button></div>
      </div>
      <section className="notice-card danger-soft">
        <strong>실행하지 않는 흐름</strong>
        <p>{plan ? plan.forbidden_commands.join(" · ") : "강제 권한 상승, 비밀번호 수집, 출처 불명 설치 명령은 실행하지 않습니다."}</p>
      </section>
    </div>
  );
}

function EmptyState({
  title,
  body,
  actionLabel,
  onAction,
  disabled,
}: {
  title: string;
  body: string;
  actionLabel?: string;
  onAction?: () => void;
  disabled?: boolean;
}) {
  return (
    <section className="empty-state">
      <strong>{title}</strong>
      <p>{body}</p>
      {actionLabel && onAction && <button type="button" onClick={onAction} disabled={disabled}>{actionLabel}</button>}
    </section>
  );
}

function ProgressModal({ task }: { task: BusyTask }) {
  const title = task === "diagnostics"
    ? "안전 진단을 진행 중입니다"
    : task === "plan"
      ? "설치 계획을 불러오는 중입니다"
      : task === "update"
        ? "업데이트를 확인하는 중입니다"
        : "리포트를 만드는 중입니다";
  const helper = task === "diagnostics"
    ? "허용된 확인 명령만 실행하고, 비밀번호나 토큰은 요청하지 않습니다."
    : task === "update"
      ? "공개 GitHub 릴리즈의 서명된 업데이트 정보만 확인합니다."
    : "잠시만 기다려 주세요. 화면을 이동해도 진행 상태는 유지됩니다.";

  return (
    <div className="modal-backdrop" role="alertdialog" aria-modal="true" aria-label="진행 상황">
      <section className="progress-modal">
        <div className="spinner" aria-hidden="true" />
        <div>
          <p className="eyebrow">진행 중</p>
          <h2>{title}</h2>
          <p>{helper}</p>
          <ol>
            <li>허용된 작업인지 확인</li>
            <li>현재 컴퓨터 상태 점검</li>
            <li>민감정보 가림 처리</li>
            <li>화면에 결과 표시</li>
          </ol>
        </div>
      </section>
    </div>
  );
}

export default App;
