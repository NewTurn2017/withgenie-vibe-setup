import { elevationMethodLabels } from "./risk";
import type { ApprovalCard } from "./types";

type LogLineKind = "info" | "stdout" | "stderr" | "system";

export type LogLine = {
  kind: LogLineKind;
  text: string;
};

export function LogView({
  focusedCard,
  lines,
}: {
  focusedCard: ApprovalCard | null;
  lines: LogLine[];
}) {
  return (
    <section className="log-view" aria-label="실행 출력 (읽기 전용)" aria-readonly="true">
      <header className="log-view-header">
        <strong>실행 출력</strong>
        <span>읽기 전용 · 입력은 받지 않습니다</span>
      </header>
      <div className="log-view-meta">
        {focusedCard ? (
          <>
            <p>예정 명령</p>
            <code>{focusedCard.step.command_preview}</code>
            <p className="log-view-elevation">권한: {elevationMethodLabels[focusedCard.step.requires_elevation_method]}</p>
          </>
        ) : (
          <p>승인 큐에서 항목을 선택하면 예정 명령과 권한 안내가 여기에 표시됩니다.</p>
        )}
      </div>
      <pre className="log-view-stream" tabIndex={-1}>
        {lines.length === 0
          ? "아직 실행된 출력이 없습니다. 승인된 레시피만 이 영역에 출력됩니다."
          : lines.map((line) => `[${line.kind}] ${line.text}`).join("\n")}
      </pre>
    </section>
  );
}
