import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ApprovalCard, ApprovalDecision, CheckStatus, ExecuteSetupActionInput, ExecutionOutcome, ExecutionStatus, HealthReport, SetupExecutionEvent, SetupPlan, ToolCheck } from "./types";
import { deriveApprovalQueue } from "./approvalQueue";
import { buildLocalHandoffPacket, formatHandoffPacket } from "./handoffPacket";
import { approvalDecisionLabels, riskTierClassName, riskTierDescriptions, riskTierLabels } from "./risk";
import { clearResumeState, loadResumeState, saveResumeState } from "./storage";
import { LogView } from "./logView";
import type { LogLine } from "./logView";
import "./App.css";

type ScreenId = "overview" | "plan" | "diagnostics" | "approval" | "report" | "help";
type BusyTask = "plan" | "diagnostics" | "report" | "update" | "execution" | null;
type StepState = "done" | "current" | "waiting";

type FlowStage = {
  id: string;
  title: string;
  helper: string;
  state: StepState;
};

type SocialLinkId = "website" | "threads" | "github";

type SocialLink = {
  id: SocialLinkId;
  label: string;
  url: string;
};

const socialLinks: SocialLink[] = [
  { id: "website", label: "홈페이지", url: "https://www.codewithgenie.com" },
  { id: "threads", label: "Threads", url: "https://www.threads.com/@ai_developer_genie" },
  { id: "github", label: "GitHub", url: "https://github.com/NewTurn2017/withgenie-vibe-setup" },
];

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


function stageBadgeLabel(state: StepState): string {
  if (state === "done") return "완료";
  if (state === "current") return "진행 중";
  return "대기";
}

function friendlyCheckLabel(check: ToolCheck): string {
  const text = `${check.id} ${check.label}`.toLowerCase();
  if (text.includes("node")) return "코딩 실행 준비";
  if (text.includes("pnpm")) return "수업 프로젝트 실행";
  if (text.includes("git.version")) return "프로젝트 저장 준비";
  if (text.includes("gh.auth") || text.includes("github") || text.includes("gh cli")) return "GitHub 연결";
  if (text.includes("vercel")) return "Vercel 연결";
  if (text.includes("vcredist") || text.includes("c++") || text.includes("webview2")) return "Windows 실행 런타임";
  if (text.includes("codex")) return "Codex 앱 준비";
  if (text.includes("supabase")) return "Supabase 연결";
  return check.label;
}

function simpleStatusMessage(check: ToolCheck): string {
  if (check.status === "installed") return "준비됐어요";
  if (check.status === "missing") return "설치가 필요해요";
  if (check.status === "needs_repair") return "복구가 필요해요";
  if (check.status === "needs_restart") return "재시작 후 확인해요";
  if (check.status === "blocked") return "도움이 필요해요";
  if (check.status === "unsupported") return "지원 확인이 필요해요";
  return "선택 항목이에요";
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function primaryActionLabelForQueue(
  cards: ApprovalCard[],
  checksCount: number,
  executionStatuses: Record<string, ExecutionStatus> = {},
): string {
  if (cards.length > 0) {
    const first = cards[0];
    if (first.step.action_phase === "external_flow" && executionStatuses[first.id] === "needs_browser_auth") return "로그인 완료 확인";
    if (first.step.action_phase === "external_flow") return "1분 점검 / 로그인 계속";
    if (first.step.action_phase === "install") return "1분 점검 / 설치 계속";
    return "1분 점검 / 다음 단계";
  }
  return checksCount > 0 ? "1분 점검 다시 하기" : "1분 점검 시작";
}

function installRollbackSteps(plan: SetupPlan | null) {
  return plan?.steps.filter((step) => step.action_phase === "install" && step.rollback_note_ko.trim().length > 0) ?? [];
}

function buildUninstallGuideText(plan: SetupPlan | null): string {
  const rollbackSteps = installRollbackSteps(plan);
  return [
    "Vibe Coding Setup 제거 안내",
    "",
    "1) 프로그램 자체 제거",
    "- Windows: 설정 > 앱 > 설치된 앱 > Vibe Coding Setup > 제거",
    "- macOS: 응용 프로그램 폴더에서 Vibe Coding Setup.app을 휴지통으로 이동",
    "",
    "2) 이 앱으로 준비한 도구 되돌리기(필요할 때만)",
    rollbackSteps.length > 0
      ? rollbackSteps.map((step) => `- ${step.label_ko}: ${step.rollback_note_ko}`).join("\n")
      : "- 설치 계획을 먼저 불러오면 도구별 제거 안내가 표시됩니다.",
    "",
    "3) 계정 연결 해제(원할 때만)",
    "- GitHub CLI: gh auth logout",
    "- Vercel CLI: vercel logout",
    "- Supabase CLI: supabase logout",
    "",
    "주의: Node.js, Git, VC++ Runtime, WebView2 Runtime은 다른 프로그램도 사용할 수 있습니다. 수업 도구만 지우려면 pnpm/Vercel/Supabase처럼 범위가 좁은 항목부터 제거하세요.",
  ].join("\n");
}

function App() {
  const initialResumeState = useMemo(() => loadResumeState(), []);
  const [activeScreen, setActiveScreen] = useState<ScreenId>(initialResumeState.activeScreen);
  const [plan, setPlan] = useState<SetupPlan | null>(null);
  const [checks, setChecks] = useState<ToolCheck[]>([]);
  const [report, setReport] = useState<HealthReport | null>(null);
  const [busyTask, setBusyTask] = useState<BusyTask>(null);
  const [message, setMessage] = useState("버튼 하나로 점검하고, 필요한 것만 순서대로 준비합니다.");
  const [approvalDecisions, setApprovalDecisions] = useState<Record<string, ApprovalDecision>>(initialResumeState.approvalDecisions);
  const [focusedCardId, setFocusedCardId] = useState<string | null>(null);
  const [logLines, setLogLines] = useState<LogLine[]>([]);
  const [executionStatuses, setExecutionStatuses] = useState<Record<string, ExecutionStatus>>({});
  const [latestExecutionEvent, setLatestExecutionEvent] = useState<SetupExecutionEvent | null>(null);
  const [modalProgressPercent, setModalProgressPercent] = useState(0);
  const [progressStartedAt, setProgressStartedAt] = useState(() => Date.now());
  const externalAuthPollsRef = useRef<Set<string>>(new Set());

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

  const approvalQueue = useMemo(
    () => deriveApprovalQueue(plan, checks, approvalDecisions),
    [approvalDecisions, checks, plan],
  );

  const focusedCard = useMemo(
    () => approvalQueue.find((card) => card.id === focusedCardId) ?? approvalQueue[0] ?? null,
    [approvalQueue, focusedCardId],
  );

  const handoffPacketText = useMemo(
    () => formatHandoffPacket(buildLocalHandoffPacket(report, checks, approvalQueue)),
    [approvalQueue, checks, report],
  );

  const currentReadiness = report ? readinessLabels[report.summary.class_readiness] : checks.length > 0 ? "리포트 준비됨" : "진단 전";
  const progressPercent = report
    ? Math.round((report.summary.required_passed / Math.max(report.summary.required_total, 1)) * 100)
    : checks.length > 0
      ? Math.round((requiredPassed / Math.max(requiredCount, 1)) * 100)
      : plan
        ? 35
        : 12;

  useEffect(() => {
    saveResumeState({
      lastUpdatedAt: new Date().toISOString(),
      lastCompletedStep: checks.length > 0 ? "diagnostics" : plan ? "plan" : "overview",
      activeScreen,
      approvalDecisions,
    });
  }, [activeScreen, approvalDecisions, checks.length, plan]);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | undefined;

    listen<SetupExecutionEvent>("setup://execution-event", (event) => {
      if (cancelled) return;
      setLatestExecutionEvent(event.payload);
      setLogLines((current) => [
        ...current,
        {
          kind: event.payload.kind,
          text: event.payload.command_preview
            ? `${event.payload.message_ko} (${event.payload.command_preview})`
            : event.payload.message_ko,
        },
      ]);
    })
      .then((unlisten) => {
        cleanup = unlisten;
      })
      .catch((error) => {
        setLogLines((current) => [
          ...current,
          { kind: "system", text: `실행 이벤트 연결에 실패했습니다: ${String(error)}` },
        ]);
      });

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    if (!busyTask) {
      setLatestExecutionEvent(null);
      setModalProgressPercent(0);
      return;
    }
    setProgressStartedAt(Date.now());
    setModalProgressPercent(initialModalProgress(busyTask));
  }, [busyTask]);

  useEffect(() => {
    if (!busyTask) return;

    const timer = window.setInterval(() => {
      const elapsedSeconds = Math.floor((Date.now() - progressStartedAt) / 1000);
      const target = modalProgressTarget(busyTask, latestExecutionEvent);
      const elapsedFloor = Math.min(target - 2, initialModalProgress(busyTask) + elapsedSeconds * 2);
      setModalProgressPercent((current) => Math.min(target, Math.max(current + 1, elapsedFloor)));
    }, 500);

    return () => window.clearInterval(timer);
  }, [busyTask, progressStartedAt, latestExecutionEvent?.status]);

  const installQueueCount = useMemo(
    () => approvalQueue.filter((card) => card.step.action_phase === "install").length,
    [approvalQueue],
  );

  const browserQueueCount = useMemo(
    () => approvalQueue.filter((card) => card.step.action_phase === "external_flow").length,
    [approvalQueue],
  );

  const flowStages = useMemo<FlowStage[]>(() => {
    const hasDiagnostics = checks.length > 0;
    const isReady = hasDiagnostics && approvalQueue.length === 0 && !!report;
    const hasInstallWork = installQueueCount > 0;
    const hasBrowserWork = browserQueueCount > 0;
    return [
      {
        id: "check",
        title: "점검",
        helper: hasDiagnostics ? `${checks.length}개 확인 완료` : "먼저 누르세요",
        state: busyTask === "diagnostics" || !hasDiagnostics ? "current" : "done",
      },
      {
        id: "install",
        title: "설치",
        helper: hasInstallWork ? `${installQueueCount}개 남음` : hasDiagnostics ? "필요 없음" : "점검 후 표시",
        state: hasInstallWork || (busyTask === "execution" && approvalQueue[0]?.step.action_phase === "install") ? "current" : hasDiagnostics ? "done" : "waiting",
      },
      {
        id: "login",
        title: "로그인",
        helper: hasBrowserWork ? `${browserQueueCount}개 연결` : hasDiagnostics ? "확인 완료" : "마지막에 진행",
        state: hasBrowserWork || (busyTask === "execution" && approvalQueue[0]?.step.action_phase === "external_flow") ? "current" : hasDiagnostics && !hasBrowserWork ? "done" : "waiting",
      },
      {
        id: "finish",
        title: "완료",
        helper: isReady ? "수업 준비 완료" : "끝나면 버튼 확인",
        state: isReady ? "done" : hasDiagnostics && approvalQueue.length === 0 ? "current" : "waiting",
      },
    ];
  }, [approvalQueue, browserQueueCount, busyTask, checks.length, installQueueCount, report]);

  const primaryFlowLabel = primaryActionLabelForQueue(approvalQueue, checks.length, executionStatuses);



  async function continuePrimaryFlow() {
    if (isBusy) return;
    if (approvalQueue[0]) {
      if (
        approvalQueue[0].step.action_phase === "external_flow"
        && executionStatuses[approvalQueue[0].id] === "needs_browser_auth"
      ) {
        await runDiagnostics();
        return;
      }
      await executeApprovalAction(approvalQueue[0], true);
      return;
    }
    await runDiagnostics();
  }

  function resetLocalProgress() {
    clearResumeState();
    setApprovalDecisions({});
    setActiveScreen("overview");
    setExecutionStatuses({});
    setMessage("이 컴퓨터의 로컬 진행 상태를 초기화했습니다. 진단 기록 파일이나 외부 계정은 삭제하지 않습니다.");
  }

  function setApprovalDecision(cardId: string, decision: ApprovalDecision) {
    setApprovalDecisions((current) => ({ ...current, [cardId]: decision }));
    setMessage(`승인 큐 항목을 '${approvalDecisionLabels[decision]}' 상태로 표시했습니다.`);
  }

  async function refreshDiagnosticsAfterExecution(
    decisions: Record<string, ApprovalDecision>,
    options: { silent?: boolean; waitingCardId?: string } = {},
  ): Promise<ApprovalCard[]> {
    const setupPlan = plan ?? await invoke<SetupPlan>("get_setup_plan");
    if (!plan) {
      setPlan(setupPlan);
    }
    setLatestExecutionEvent((current) => current
      ? {
          ...current,
          status: "verifying",
          kind: "system",
          message_ko: "방금 끝난 작업을 반영해 전체 상태를 다시 점검합니다.",
        }
      : current);
    const nextChecks = await invoke<ToolCheck[]>("run_all_diagnostics");
    setChecks(nextChecks);
    const nextReport = await invoke<HealthReport>("build_health_report", {
      input: { checks: nextChecks },
    });
    setReport(nextReport);

    const nextQueue = deriveApprovalQueue(setupPlan, nextChecks, decisions);
    setFocusedCardId(nextQueue[0]?.id ?? null);
    if (nextQueue.length > 0) {
      setActiveScreen("approval");
      if (options.silent && options.waitingCardId && nextQueue.some((card) => card.id === options.waitingCardId)) {
        setMessage("아직 로그인 완료가 확인되지 않았습니다. 브라우저/명령 창에서 인증을 끝내면 자동으로 다시 확인합니다.");
      } else {
        setMessage(`${nextQueue.length}개 항목이 남았습니다. 다음 항목으로 계속 진행할 수 있습니다.`);
      }
    } else {
      setActiveScreen("diagnostics");
      setMessage(nextReport.summary.beginner_message);
    }

    return nextQueue;
  }

  async function loadPlan() {
    setBusyTask("plan");
    setActiveScreen("plan");
    setMessage("설치 계획을 불러오는 중입니다...");
    try {
      const setupPlan = await invoke<SetupPlan>("get_setup_plan");
      setPlan(setupPlan);
      setMessage("설치 계획을 불러왔습니다. 필요한 작업만 순서대로 보여드립니다.");
    } catch (error) {
      setMessage(`설치 계획을 불러오지 못했습니다: ${String(error)}`);
    } finally {
      setBusyTask(null);
    }
  }

  async function executeApprovalAction(
    card: ApprovalCard,
    autoContinue = false,
    decisionsBase: Record<string, ApprovalDecision> = approvalDecisions,
  ) {
    setFocusedCardId(card.id);
    const nextDecisions: Record<string, ApprovalDecision> = { ...decisionsBase, [card.id]: "approved" };
    setApprovalDecisions(nextDecisions);
    setMessage(`승인 큐 항목을 '${approvalDecisionLabels.approved}' 상태로 표시했습니다. 이제 실제 실행/검증 상태를 따로 추적합니다.`);
    setExecutionStatuses((current) => ({ ...current, [card.id]: "running" }));
    setBusyTask("execution");
    setProgressStartedAt(Date.now());
    setModalProgressPercent(initialModalProgress("execution"));
    setLatestExecutionEvent({
      action_id: card.step.id,
      status: "queued",
      kind: "system",
      message_ko: `${card.step.label_ko} 준비 중입니다.`,
      command_preview: card.step.command_preview,
      docs_url: card.step.docs_url,
    });
    setLogLines((current) => [
      ...current,
      { kind: "system", text: `${card.step.label_ko} 작업을 준비합니다.` },
    ]);

    try {
      const input: ExecuteSetupActionInput = {
        action_id: card.step.id,
        approval_id: card.id,
      };
      const outcome = await invoke<ExecutionOutcome>("execute_setup_action", { input });
      setExecutionStatuses((current) => ({ ...current, [card.id]: outcome.status }));
      setMessage(outcome.message_ko);
      setLogLines((current) => [
        ...current,
        { kind: outcome.status === "blocked" ? "stderr" : "system", text: outcome.next_action_ko },
      ]);
      if (outcome.status === "done" || outcome.status === "needs_reboot") {
        const nextQueue = await refreshDiagnosticsAfterExecution(nextDecisions);
        if (autoContinue && outcome.status === "done") {
          const nextInstallCard = nextQueue.find((nextCard) => nextCard.step.action_phase === "install");
          if (nextInstallCard) {
            await executeApprovalAction(nextInstallCard, true, nextDecisions);
          } else if (nextQueue.length > 0) {
            setFocusedCardId(nextQueue[0].id);
            setMessage("설치 단계는 끝났습니다. 이제 브라우저 가입/로그인처럼 사용자가 직접 확인해야 하는 단계가 남았습니다.");
          }
        }
      } else if (outcome.status === "needs_browser_auth") {
        setMessage("브라우저/명령 창에서 로그인을 완료하세요. 완료되면 앱이 자동으로 다시 확인하고, 바로 확인하려면 '로그인 완료 확인'을 누르세요.");
        void pollExternalAuthCompletion(card, nextDecisions);
      }
    } catch (error) {
      setExecutionStatuses((current) => ({ ...current, [card.id]: "blocked" }));
      setMessage(`작업을 시작하지 못했습니다: ${String(error)}`);
      setLogLines((current) => [
        ...current,
        { kind: "stderr", text: `작업 시작 실패: ${String(error)}` },
      ]);
    } finally {
      setBusyTask(null);
    }
  }

  async function pollExternalAuthCompletion(card: ApprovalCard, decisions: Record<string, ApprovalDecision>) {
    if (externalAuthPollsRef.current.has(card.id)) {
      return;
    }

    externalAuthPollsRef.current.add(card.id);
    try {
      for (let attempt = 0; attempt < 36; attempt += 1) {
        await sleep(5000);
        const nextQueue = await refreshDiagnosticsAfterExecution(decisions, {
          silent: true,
          waitingCardId: card.id,
        });
        if (!nextQueue.some((nextCard) => nextCard.id === card.id)) {
          setExecutionStatuses((current) => ({ ...current, [card.id]: "done" }));
          setMessage(`${friendlyCheckLabel(card.check)} 로그인이 확인되었습니다. 해당 버튼은 이제 사라집니다.`);
          return;
        }
      }
      setMessage("자동 확인 시간이 끝났습니다. 로그인 완료 후 '로그인 완료 확인'을 눌러 다시 확인하세요.");
    } catch (error) {
      setMessage(`로그인 완료 자동 확인 중 문제가 생겼습니다. '로그인 완료 확인'을 눌러 다시 확인하세요: ${String(error)}`);
    } finally {
      externalAuthPollsRef.current.delete(card.id);
    }
  }

  async function runDiagnostics() {
    setBusyTask("diagnostics");
    setProgressStartedAt(Date.now());
    setModalProgressPercent(initialModalProgress("diagnostics"));
    setActiveScreen("diagnostics");
    setMessage("진단 중입니다. 허용된 확인 명령만 실행합니다...");
    try {
      const setupPlan = plan ?? await invoke<SetupPlan>("get_setup_plan");
      if (!plan) {
        setPlan(setupPlan);
      }
      const nextChecks = await invoke<ToolCheck[]>("run_all_diagnostics");
      setChecks(nextChecks);
      const nextReport = await invoke<HealthReport>("build_health_report", {
        input: { checks: nextChecks },
      });
      setReport(nextReport);
      const nextQueue = deriveApprovalQueue(setupPlan, nextChecks, approvalDecisions);
      if (nextQueue.length > 0) {
        setActiveScreen("approval");
        setMessage(`${nextQueue.length}개 항목에 대해 다음 행동을 선택해야 합니다.`);
      } else {
        setMessage(nextReport.summary.beginner_message);
      }
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

  async function copyHandoffPacket() {
    try {
      await navigator.clipboard.writeText(handoffPacketText);
      setMessage("강사용 핸드오프 패킷을 클립보드에 복사했습니다.");
    } catch (error) {
      setMessage(`핸드오프 패킷을 복사하지 못했습니다: ${String(error)}`);
    }
  }

  async function copyUninstallGuide() {
    try {
      const setupPlan = plan ?? await invoke<SetupPlan>("get_setup_plan");
      if (!plan) {
        setPlan(setupPlan);
      }
      await navigator.clipboard.writeText(buildUninstallGuideText(setupPlan));
      setMessage("제거 안내를 클립보드에 복사했습니다. 필요한 항목만 골라서 실행하세요.");
    } catch (error) {
      setMessage(`제거 안내를 복사하지 못했습니다: ${String(error)}`);
    }
  }

  async function openUninstallSettings() {
    try {
      const result = await invoke<string>("open_uninstall_settings");
      setMessage(result);
    } catch (error) {
      setMessage(`앱 제거 설정을 열지 못했습니다: ${String(error)}`);
    }
  }

  async function openExternalLink(link: SocialLink) {
    const hasTauriRuntime = Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
    try {
      if (hasTauriRuntime) {
        await openUrl(link.url);
      } else {
        window.open(link.url, "_blank", "noopener,noreferrer");
      }
      setMessage(`${link.label}를 기본 브라우저로 열었습니다.`);
    } catch (error) {
      setMessage(`${link.label} 링크를 열지 못했습니다: ${String(error)}`);
    }
  }

  async function checkForUpdates() {
    setBusyTask("update");
    setActiveScreen("help");
    setMessage("GitHub 릴리즈에서 새 버전을 확인하는 중입니다...");
    try {
      const update = await check({ timeout: 8000 });
      if (!update) {
        setMessage("현재 설치된 Vibe Coding Setup이 최신 버전입니다.");
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
          <p className="eyebrow">WithGenie에서 만든 수업 준비 도우미</p>
          <h1>Vibe Coding Setup</h1>
          <p>어려운 용어는 줄이고, 준비 흐름만 크게 보여줍니다.</p>
        </div>
        <div className="top-actions" aria-label="주요 작업">
          <button type="button" className="primary big-primary" onClick={continuePrimaryFlow} disabled={isBusy}>
            {busyTask === "diagnostics" ? "점검 중" : busyTask === "execution" ? "진행 중" : primaryFlowLabel}
          </button>
          <button type="button" onClick={() => setActiveScreen("help")} disabled={isBusy}>도움말</button>
        </div>
      </header>

      <section className="status-strip" role="status" aria-live="polite">
        <div>
          <span className="status-label">지금 할 일</span>
          <strong>{message}</strong>
        </div>
        <div className="status-meters" aria-label="진행 요약">
          <span>준비도 {progressPercent}%</span>
          <span>필수 {requiredPassed}/{requiredCount || "-"}</span>
          <span>남은 작업 {approvalQueue.length || needsActionCount}</span>
          <span>{currentReadiness}</span>
        </div>
      </section>

      <div className="workspace">
        <aside className="side-rail" aria-label="설치 흐름">
          <FlowDiagram stages={flowStages} />

          <div className="summary-card">
            <span className={`readiness ${report?.summary.class_readiness ?? "pending"}`}>{currentReadiness}</span>
            <p>{report?.summary.beginner_message ?? "먼저 1분 점검을 누르면 다음 할 일을 알려드립니다."}</p>
            <div className="progress-bar" aria-label={`준비도 ${progressPercent}%`}>
              <span style={{ width: `${progressPercent}%` }} />
            </div>
            <div className="side-actions">
              <button type="button" className="primary" onClick={continuePrimaryFlow} disabled={isBusy}>{primaryFlowLabel}</button>
              <button type="button" onClick={() => setActiveScreen("report")} disabled={checks.length === 0 || isBusy}>문제 공유</button>
            </div>
          </div>
        </aside>

        <section className="screen-card" aria-label="선택한 화면">
          {activeScreen === "overview" && renderOverview(flowStages, continuePrimaryFlow, primaryFlowLabel, isBusy)}
          {activeScreen === "plan" && renderPlan(plan, loadPlan, isBusy)}
          {activeScreen === "diagnostics" && renderDiagnostics(checks, buildReport, isBusy)}
          {activeScreen === "approval" && renderApprovalQueue(approvalQueue, focusedCard, logLines, executionStatuses, setApprovalDecision, executeApprovalAction, setFocusedCardId, runDiagnostics, busyTask === "execution")}
          {activeScreen === "report" && renderReport(report, checks, handoffPacketText, buildReport, copyReport, copyHandoffPacket, isBusy)}
          {activeScreen === "help" && renderHelp(plan, checkForUpdates, resetLocalProgress, openUninstallSettings, copyUninstallGuide, isBusy)}
        </section>
      </div>

      <footer className="app-footer">
        <span>WithGenie 제작 · 비밀번호와 토큰은 받지 않음</span>
        <div className="footer-links" aria-label="WithGenie 링크">
          {socialLinks.map((link) => (
            <button
              type="button"
              className={`icon-link ${link.id}`}
              key={link.id}
              onClick={() => openExternalLink(link)}
              title={`${link.label} 열기`}
              aria-label={`${link.label} 열기`}
            >
              <SocialIcon id={link.id} />
            </button>
          ))}
        </div>
      </footer>

      {busyTask && (
        <ProgressModal
          task={busyTask}
          card={focusedCard}
          event={latestExecutionEvent}
          percent={modalProgressPercent}
        />
      )}
    </main>
  );
}

function renderOverview(
  stages: FlowStage[],
  onPrimary: () => void,
  primaryLabel: string,
  isBusy: boolean,
) {
  return (
    <div className="screen-stack overview-stack simple-overview">
      <section className="screen-hero compact minimal-hero hero-modern">
        <div>
          <p className="eyebrow">쉬운 수업 준비</p>
          <h2>버튼 하나씩만 누르면 됩니다.</h2>
          <p>앱이 점검하고, 설치가 필요하면 순서대로 안내합니다.</p>
        </div>
        <button type="button" className="primary big-primary" onClick={onPrimary} disabled={isBusy}>{primaryLabel}</button>
      </section>

      <FlowDiagram stages={stages} large />

      <section className="quick-grid minimal-grid">
        <article className="info-card calm-card">
          <span className="card-kicker">✓</span>
          <strong>진행 중인지 보임</strong>
          <p>각 단계에 완료 배지가 붙습니다.</p>
        </article>
        <article className="info-card calm-card">
          <span className="card-kicker">↗</span>
          <strong>로그인은 공식 화면</strong>
          <p>GitHub와 Vercel 비밀번호는 받지 않습니다.</p>
        </article>
        <article className="info-card calm-card">
          <span className="card-kicker">?</span>
          <strong>막히면 공유</strong>
          <p>강사에게 보낼 요약을 바로 만들 수 있습니다.</p>
        </article>
      </section>
    </div>
  );
}

function FlowDiagram({ stages, large = false }: { stages: FlowStage[]; large?: boolean }) {
  return (
    <nav className={`flow-diagram ${large ? "large" : ""}`} aria-label="전체 설치 흐름">
      <div className="rail-title">
        <strong>전체 흐름</strong>
        <span>완료 배지로 확인</span>
      </div>
      <ol>
        {stages.map((stage, index) => (
          <li className={`flow-node ${stage.state}`} key={stage.id}>
            <span className="flow-index">{stage.state === "done" ? "✓" : index + 1}</span>
            <div>
              <strong>{stage.title}</strong>
              <small>{stage.helper}</small>
            </div>
            <em>{stageBadgeLabel(stage.state)}</em>
          </li>
        ))}
      </ol>
    </nav>
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
          <h2>복잡한 출력 대신 준비 상태만 보여줍니다.</h2>
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
                <strong>{friendlyCheckLabel(check)}</strong>
                <span>{statusLabels[check.status]}</span>
              </div>
              <p className="simple-result-message">{simpleStatusMessage(check)}</p>
              <small>{check.beginner_message}</small>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

function renderApprovalQueue(
  cards: ApprovalCard[],
  focusedCard: ApprovalCard | null,
  lines: LogLine[],
  executionStatuses: Record<string, ExecutionStatus>,
  setDecision: (cardId: string, decision: ApprovalDecision) => void,
  executeAction: (card: ApprovalCard, autoContinue?: boolean) => void,
  setFocused: (cardId: string) => void,
  rerunDiagnostics: () => void,
  isExecuting: boolean,
) {
  const firstCard = cards[0];
  const firstNeedsAuthCheck = firstCard?.step.action_phase === "external_flow" && executionStatuses[firstCard.id] === "needs_browser_auth";

  return (
    <div className="screen-stack approval-stack">
      <div className="screen-heading">
        <div>
          <p className="eyebrow">다음 할 일</p>
          <h2>필요한 것만 순서대로 진행합니다.</h2>
        </div>
        <div className="inline-actions">
          <button type="button" onClick={rerunDiagnostics} disabled={isExecuting}>1분 점검 다시 하기</button>
          <button
            type="button"
            className="primary"
            disabled={cards.length === 0 || isExecuting}
            onClick={() => firstNeedsAuthCheck ? rerunDiagnostics() : firstCard && executeAction(firstCard, true)}
          >
            {firstNeedsAuthCheck ? "로그인 완료 확인" : "남은 작업 계속하기"}
          </button>
        </div>
      </div>

      {cards.length === 0 ? (
        <EmptyState
          title="승인이 필요한 항목이 없습니다."
          body="진단을 실행하면 설치 필요, 복구 필요, 차단 항목이 승인 큐로 정리됩니다."
        />
      ) : (
        <div className="content-scroll approval-list">
          {cards.map((card) => {
            const executionStatus = executionStatuses[card.id];
            const waitingForExternalAuth = card.step.action_phase === "external_flow" && executionStatus === "needs_browser_auth";
            return (
            <article className={`approval-card ${riskTierClassName(card.step.risk_tier)}`} key={card.id} onClick={() => setFocused(card.id)}>
              <div className="approval-card-header">
                <div>
                  <strong>{card.step.label_ko}</strong>
                  <p>{card.reason_ko}</p>
                </div>
                <span>{riskTierLabels[card.step.risk_tier]}</span>
              </div>
              <p>{waitingForExternalAuth ? "로그인 완료 여부를 자동으로 다시 확인 중입니다. 바로 확인하려면 아래 버튼을 누르세요." : card.step.requires_browser ? "브라우저가 열리면 로그인만 완료하세요." : "설치가 끝나면 앱이 다시 확인합니다."}</p>
              <p className="risk-description">{riskTierDescriptions[card.step.risk_tier]}</p>
              <div className="approval-actions">
                <button type="button" className="primary" disabled={isExecuting} onClick={() => waitingForExternalAuth ? rerunDiagnostics() : executeAction(card)}>{primaryApprovalActionLabel(card, executionStatus)}</button>
                <button type="button" onClick={() => setDecision(card.id, "deferred")}>나중에</button>
                <button type="button" onClick={() => setDecision(card.id, "ask_instructor")}>도움 요청</button>
              </div>
              <small>상태: {executionStatus ? executionStatusLabels[executionStatus] : "아직 시작 전"}</small>
            </article>
            );
          })}
        </div>
      )}
      <details className="advanced-log">
        <summary>자세한 실행 기록 보기</summary>
        <LogView focusedCard={focusedCard} lines={lines} />
      </details>
    </div>
  );
}

const executionStatusLabels: Record<ExecutionStatus, string> = {
  queued: "대기 중",
  needs_user_confirm: "사용자 확인 필요",
  running: "실행 중",
  needs_os_consent: "권한 창 확인 필요",
  needs_browser_auth: "브라우저 로그인 필요",
  needs_reboot: "재시작 후 확인 필요",
  verifying: "검증 중",
  done: "완료",
  blocked: "막힘",
};

function primaryApprovalActionLabel(card: ApprovalCard, executionStatus?: ExecutionStatus): string {
  if (card.step.action_phase === "external_flow") {
    if (executionStatus === "needs_browser_auth") {
      return "로그인 완료 확인";
    }
    return "브라우저 가입/로그인 시작";
  }

  if (card.step.action_phase === "install") {
    return card.step.risk_tier === "permission_prompt"
      ? "설치 시작 (권한 창 Yes)"
      : "설치 시작";
  }

  if (card.step.risk_tier === "permission_prompt") {
    return "권한 창에서 Yes로 진행";
  }

  if (card.step.action_phase === "detect") {
    return "원클릭 작업 준비";
  }

  return "다음 단계 시작";
}

function renderReport(
  report: HealthReport | null,
  checks: ToolCheck[],
  handoffPacketText: string,
  buildReport: () => void,
  copyReport: () => void,
  copyHandoffPacket: () => void,
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
          <button type="button" onClick={copyHandoffPacket} disabled={checks.length === 0 || isBusy}>강사용 패킷 복사</button>
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
          <article className="handoff-preview">
            <strong>강사용 핸드오프 미리보기</strong>
            <pre>{handoffPacketText}</pre>
          </article>
        </div>
      )}
    </div>
  );
}

function renderHelp(
  plan: SetupPlan | null,
  checkForUpdates: () => void,
  resetLocalProgress: () => void,
  openUninstallSettings: () => void,
  copyUninstallGuide: () => void,
  isBusy: boolean,
) {
  const rollbackSteps = installRollbackSteps(plan);

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
        <div><strong>업데이트</strong><p>새 버전이 있는지 확인합니다.</p><button type="button" onClick={checkForUpdates} disabled={isBusy}>업데이트 확인</button></div>
        <div>
          <strong>프로그램 제거</strong>
          <p>Vibe Coding Setup 자체를 지울 때는 운영체제의 앱 제거 화면을 엽니다. 설치한 개발 도구는 자동으로 지우지 않습니다.</p>
          <div className="button-row">
            <button type="button" onClick={openUninstallSettings} disabled={isBusy}>앱 제거 화면 열기</button>
            <button type="button" onClick={copyUninstallGuide} disabled={isBusy}>제거 안내 복사</button>
          </div>
        </div>
        <div className="uninstall-card">
          <strong>설치 도구 되돌리기</strong>
          <p>Node.js나 Git은 다른 앱도 쓸 수 있어 자동 삭제하지 않습니다. 필요할 때만 아래 안내를 보고 직접 제거하세요.</p>
          <details>
            <summary>도구별 제거 방법 보기</summary>
            {rollbackSteps.length > 0 ? (
              <ul className="rollback-list">
                {rollbackSteps.map((step) => (
                  <li key={step.id}>
                    <strong>{step.label_ko}</strong>
                    <span>{step.rollback_note_ko}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p>설치 계획을 먼저 불러오면 제거 안내가 표시됩니다.</p>
            )}
          </details>
        </div>
        <div><strong>로컬 진행 초기화</strong><p>승인 큐 선택과 마지막 화면 기억만 지웁니다. 설치된 도구나 계정 로그인은 건드리지 않습니다.</p><button type="button" onClick={resetLocalProgress} disabled={isBusy}>진행 상태 초기화</button></div>
      </div>
      <section className="notice-card danger-soft">
        <strong>실행하지 않는 흐름</strong>
        <p>{plan ? plan.forbidden_commands.join(" · ") : "강제 권한 상승, 비밀번호 수집, 출처 불명 설치 명령은 실행하지 않습니다."}</p>
      </section>
    </div>
  );
}

function SocialIcon({ id }: { id: SocialLinkId }) {
  if (id === "website") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="8.5" />
        <path d="M3.8 12h16.4M12 3.5c2.2 2.3 3.4 5 3.4 8.5s-1.2 6.2-3.4 8.5C9.8 18.2 8.6 15.5 8.6 12S9.8 5.8 12 3.5Z" />
      </svg>
    );
  }

  if (id === "threads") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M16.7 8.6c-.6-2.5-2.3-3.8-4.8-3.8-3.4 0-5.3 2.5-5.3 7.1s2 7.3 5.6 7.3c3 0 5.1-1.7 5.1-4.1 0-2.1-1.5-3.4-4.1-3.4h-1.1" />
        <path d="M12 11.7c-1.8 0-3 .8-3 2.1s1.1 2.1 2.7 2.1c2.2 0 3.4-1.4 3.4-3.6v-1.1c0-3.1-1.6-4.7-4.3-4.7" />
      </svg>
    );
  }

  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 3.6a8.4 8.4 0 0 0-2.7 16.4c.4.1.5-.2.5-.4v-1.6c-2.2.5-2.7-.9-2.7-.9-.4-.9-.9-1.1-.9-1.1-.7-.5.1-.5.1-.5.8.1 1.3.9 1.3.9.7 1.3 1.9.9 2.3.7.1-.5.3-.9.5-1.1-1.8-.2-3.6-.9-3.6-4a3.1 3.1 0 0 1 .8-2.2 2.9 2.9 0 0 1 .1-2.1s.7-.2 2.3.8a7.7 7.7 0 0 1 4.1 0c1.6-1 2.3-.8 2.3-.8.4 1 .2 1.8.1 2.1.5.6.8 1.3.8 2.2 0 3.1-1.9 3.8-3.6 4 .3.3.6.8.6 1.6v2.2c0 .2.1.5.6.4A8.4 8.4 0 0 0 12 3.6Z" />
    </svg>
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

function initialModalProgress(task: NonNullable<BusyTask>): number {
  if (task === "execution") return 8;
  if (task === "diagnostics") return 12;
  if (task === "update") return 16;
  return 18;
}

function modalProgressTarget(task: NonNullable<BusyTask>, event: SetupExecutionEvent | null): number {
  if (task !== "execution") {
    return task === "diagnostics" ? 88 : 82;
  }

  switch (event?.status) {
    case "queued":
      return 22;
    case "running":
      return 48;
    case "needs_os_consent":
      return 64;
    case "verifying":
      return 88;
    case "needs_browser_auth":
    case "needs_reboot":
    case "done":
    case "blocked":
      return 100;
    default:
      return 58;
  }
}

function progressStepState(index: number, currentIndex: number): StepState {
  if (index < currentIndex) return "done";
  if (index === currentIndex) return "current";
  return "waiting";
}

function modalCurrentStepIndex(task: NonNullable<BusyTask>, event: SetupExecutionEvent | null): number {
  if (task !== "execution") {
    return Math.min(2, Math.floor(modalProgressTarget(task, event) / 32));
  }

  switch (event?.status) {
    case "queued":
    case "running":
      return 1;
    case "needs_os_consent":
      return 2;
    case "verifying":
      return 3;
    case "needs_browser_auth":
    case "needs_reboot":
    case "done":
    case "blocked":
      return 4;
    default:
      return 1;
  }
}

function progressModalCopy(
  task: NonNullable<BusyTask>,
  card: ApprovalCard | null,
  event: SetupExecutionEvent | null,
) {
  const actionLabel = card?.step.label_ko ?? "다음 단계";
  const isExternalFlow = card?.step.action_phase === "external_flow";
  const isDownloadOrInstall = card?.step.action_phase === "install";

  if (task === "execution") {
    return {
      title: isExternalFlow ? `${actionLabel} 창을 여는 중입니다` : `${actionLabel} 진행 중입니다`,
      helper: event?.message_ko ?? "설치나 브라우저 로그인이 끝나면 앱이 검증 단계로 넘어갑니다.",
      detail: isExternalFlow
        ? "공식 로그인 화면만 사용합니다. 이 앱은 비밀번호나 토큰을 입력받지 않습니다."
        : isDownloadOrInstall
          ? "다운로드, 설치, PATH 반영, 검증을 순서대로 확인하고 있습니다."
          : "허용된 작업인지 확인하고 결과를 반영합니다.",
      steps: [
        "허용된 작업 확인",
        isExternalFlow ? "공식 로그인 창 열기" : "다운로드/설치 진행",
        isExternalFlow ? "사용자 로그인 완료 대기" : "설치 결과 확인",
        "전체 상태 다시 점검",
        "화면에 결과 반영",
      ],
    };
  }

  if (task === "diagnostics") {
    return {
      title: "1분 점검을 진행 중입니다",
      helper: "현재 컴퓨터에 필요한 도구가 있는지 안전하게 확인하고 있습니다.",
      detail: "비밀번호나 토큰은 요청하지 않고, 결과에 필요한 민감정보는 가려서 표시합니다.",
      steps: ["점검 계획 확인", "도구 상태 확인", "로그인 상태 확인", "결과 정리", "다음 버튼 표시"],
    };
  }

  if (task === "update") {
    return {
      title: "업데이트를 확인하는 중입니다",
      helper: "공개 GitHub 릴리즈의 서명된 업데이트 정보만 확인합니다.",
      detail: "새 버전이 있으면 사용자가 직접 진행할 수 있도록 안내합니다.",
      steps: ["업데이트 채널 확인", "릴리즈 정보 확인", "서명 확인", "결과 정리", "화면에 표시"],
    };
  }

  return {
    title: task === "plan" ? "설치 계획을 불러오는 중입니다" : "리포트를 만드는 중입니다",
    helper: "잠시만 기다려 주세요. 화면을 이동해도 진행 상태는 유지됩니다.",
    detail: "사용자에게 필요한 다음 행동만 간단히 보여주도록 정리합니다.",
    steps: ["안전 규칙 확인", "필요 항목 정리", "민감정보 가림 처리", "결과 정리", "화면에 표시"],
  };
}

function ProgressModal({
  task,
  card,
  event,
  percent,
}: {
  task: NonNullable<BusyTask>;
  card: ApprovalCard | null;
  event: SetupExecutionEvent | null;
  percent: number;
}) {
  const copy = progressModalCopy(task, card, event);
  const currentStepIndex = modalCurrentStepIndex(task, event);
  const clampedPercent = Math.max(0, Math.min(100, Math.round(percent)));

  return (
    <div className="modal-backdrop" role="alertdialog" aria-modal="true" aria-label="진행 상황">
      <section className="progress-modal">
        <div className="spinner" aria-hidden="true" />
        <div className="progress-modal-copy">
          <p className="eyebrow">진행 중 · {clampedPercent}%</p>
          <h2>{copy.title}</h2>
          <p>{copy.helper}</p>
          <p className="progress-detail">{copy.detail}</p>
          <div className="progress-track" role="progressbar" aria-label={copy.title} aria-valuemin={0} aria-valuemax={100} aria-valuenow={clampedPercent}>
            <span style={{ width: `${clampedPercent}%` }} />
          </div>
          <ol className="progress-steps">
            {copy.steps.map((step, index) => {
              const state = progressStepState(index, currentStepIndex);
              return (
                <li key={step} className={`progress-step ${state}`}>
                  <span>{index + 1}</span>
                  {step}
                </li>
              );
            })}
          </ol>
        </div>
      </section>
    </div>
  );
}

export default App;
