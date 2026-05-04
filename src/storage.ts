import type { ResumeState } from "./types";

const STORAGE_KEY = "withgenie.vibeSetup.resumeState.v1";

export const defaultResumeState: ResumeState = {
  lastUpdatedAt: new Date(0).toISOString(),
  lastCompletedStep: "overview",
  activeScreen: "overview",
  approvalDecisions: {},
};

export function loadResumeState(): ResumeState {
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return defaultResumeState;
  }

  try {
    const parsed = JSON.parse(raw) as Partial<ResumeState>;
    return {
      lastUpdatedAt: typeof parsed.lastUpdatedAt === "string" ? parsed.lastUpdatedAt : defaultResumeState.lastUpdatedAt,
      lastCompletedStep: parsed.lastCompletedStep ?? defaultResumeState.lastCompletedStep,
      // Plan, diagnostics, approval queue, and report data are rebuilt at runtime.
      // Until those objects are persisted too, restoring a deep screen creates
      // empty approval/report states on fresh app launch.
      activeScreen: defaultResumeState.activeScreen,
      approvalDecisions: parsed.approvalDecisions ?? {},
    };
  } catch {
    return defaultResumeState;
  }
}

export function saveResumeState(state: ResumeState): void {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

export function clearResumeState(): void {
  window.localStorage.removeItem(STORAGE_KEY);
}
