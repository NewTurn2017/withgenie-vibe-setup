# Health Report Schema: 원클릭 개발 환경 GUI 설치기

- 작성일: 2026-05-01
- 상태: Draft for gate review
- 관련 PRD: `.omx/plans/prd-one-click-dev-setup-installer.md`
- 관련 recipe matrix: `docs/recipe-matrix.md`
- 목적: 강사/지원자가 참가자의 수업 가능 여부를 빠르게 판단하고, raw diagnostics를 안전하게 전달받기 위한 report 구조 정의

## 1. 설계 원칙

1. 보고서는 사람용 summary와 기계 판독 가능한 raw diagnostics를 함께 가진다.
2. credential, token, password, email, 사용자 홈 경로 등은 redaction한다.
3. 각 tool check는 `installed`, `missing`, `needs_repair`, `needs_restart`, `optional_skipped`, `unsupported`, `blocked` 중 하나의 status를 가진다.
4. `blocked`, `unsupported`, `needs_repair`는 beginner message와 support action을 가져야 한다.
5. MVP에서는 report를 로컬 파일 export만 하며 외부 telemetry 전송은 하지 않는다.

## 2. Report summary states

| 상태 | 의미 |
| --- | --- |
| `ready_for_class` | 필수 항목 통과, 선택 항목은 installed 또는 optional_skipped |
| `needs_attention` | 복구 가능한 needs_repair/needs_restart 존재 |
| `blocked` | 권한/정책/네트워크/unsupported로 강사 지원 필요 |
| `unsupported` | OS/정책상 MVP 지원 불가 |

## 3. JSON Schema draft

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://raw.githubusercontent.com/NewTurn2017/withgenie-vibe-setup/main/schemas/health-report.schema.json",
  "title": "VibeSetupHealthReport",
  "type": "object",
  "required": [
    "schema_version",
    "generated_at",
    "app",
    "machine",
    "summary",
    "checks",
    "redaction"
  ],
  "additionalProperties": false,
  "properties": {
    "schema_version": {
      "type": "string",
      "const": "0.1.0"
    },
    "generated_at": {
      "type": "string",
      "format": "date-time"
    },
    "app": {
      "type": "object",
      "required": ["name", "version", "recipe_version", "distribution_channel"],
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string" },
        "version": { "type": "string" },
        "recipe_version": { "type": "string" },
        "distribution_channel": {
          "type": "string",
          "enum": ["private_mvp", "public_github", "signed_beta", "production"]
        }
      }
    },
    "machine": {
      "type": "object",
      "required": ["os", "os_version", "arch", "shell", "network"],
      "additionalProperties": false,
      "properties": {
        "os": { "type": "string", "enum": ["macos", "windows"] },
        "os_version": { "type": "string" },
        "build_number": { "type": "string" },
        "arch": { "type": "string", "enum": ["arm64", "x64", "unknown"] },
        "shell": { "type": "string" },
        "network": {
          "type": "object",
          "required": ["status"],
          "additionalProperties": false,
          "properties": {
            "status": { "type": "string", "enum": ["online", "limited", "offline", "unknown"] },
            "blocked_hosts": {
              "type": "array",
              "items": { "type": "string" }
            }
          }
        }
      }
    },
    "summary": {
      "type": "object",
      "required": ["class_readiness", "required_passed", "required_total", "needs_instructor_help", "beginner_message"],
      "additionalProperties": false,
      "properties": {
        "class_readiness": {
          "type": "string",
          "enum": ["ready_for_class", "needs_attention", "blocked", "unsupported"]
        },
        "required_passed": { "type": "integer", "minimum": 0 },
        "required_total": { "type": "integer", "minimum": 0 },
        "needs_instructor_help": { "type": "boolean" },
        "beginner_message": { "type": "string" },
        "instructor_message": { "type": "string" }
      }
    },
    "checks": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/check" }
    },
    "execution_log": {
      "type": "array",
      "items": { "$ref": "#/$defs/log_entry" }
    },
    "resume_state": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "last_completed_step": { "type": "string" },
        "next_action": { "type": "string" },
        "requires_restart": { "type": "boolean" }
      }
    },
    "redaction": {
      "type": "object",
      "required": ["applied", "rules_version", "masked_fields"],
      "additionalProperties": false,
      "properties": {
        "applied": { "type": "boolean", "const": true },
        "rules_version": { "type": "string" },
        "masked_fields": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    }
  },
  "$defs": {
    "check": {
      "type": "object",
      "required": [
        "id",
        "label",
        "required_for_class",
        "status",
        "beginner_message",
        "support_action"
      ],
      "additionalProperties": false,
      "properties": {
        "id": { "type": "string" },
        "label": { "type": "string" },
        "required_for_class": { "type": "boolean" },
        "status": {
          "type": "string",
          "enum": ["installed", "missing", "needs_repair", "needs_restart", "optional_skipped", "unsupported", "blocked"]
        },
        "detected_version": { "type": "string" },
        "required_version": { "type": "string" },
        "verify_command_label": { "type": "string" },
        "beginner_message": { "type": "string" },
        "support_action": { "type": "string" },
        "evidence": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "exit_code": { "type": "integer" },
            "duration_ms": { "type": "integer", "minimum": 0 },
            "stdout_redacted": { "type": "string" },
            "stderr_redacted": { "type": "string" }
          }
        },
        "links": {
          "type": "array",
          "items": { "type": "string", "format": "uri" }
        }
      }
    },
    "log_entry": {
      "type": "object",
      "required": ["timestamp", "step_id", "command_label", "status"],
      "additionalProperties": false,
      "properties": {
        "timestamp": { "type": "string", "format": "date-time" },
        "step_id": { "type": "string" },
        "command_label": { "type": "string" },
        "status": { "type": "string", "enum": ["started", "passed", "failed", "cancelled", "skipped"] },
        "exit_code": { "type": "integer" },
        "duration_ms": { "type": "integer", "minimum": 0 },
        "message_redacted": { "type": "string" }
      }
    }
  }
}
```

## 4. Example report

```json
{
  "schema_version": "0.1.0",
  "generated_at": "2026-05-01T09:00:00Z",
  "app": {
    "name": "Vibe Setup",
    "version": "0.1.0-private",
    "recipe_version": "2026.05.01",
    "distribution_channel": "public_github"
  },
  "machine": {
    "os": "macos",
    "os_version": "14.7.1",
    "arch": "arm64",
    "shell": "zsh",
    "network": {
      "status": "online",
      "blocked_hosts": []
    }
  },
  "summary": {
    "class_readiness": "ready_for_class",
    "required_passed": 6,
    "required_total": 6,
    "needs_instructor_help": false,
    "beginner_message": "수업에 필요한 기본 도구가 준비되었습니다.",
    "instructor_message": "필수 항목 통과, WSL은 선택 항목입니다."
  },
  "checks": [
    {
      "id": "node.macos.v24",
      "label": "Node.js v24 LTS",
      "required_for_class": true,
      "status": "installed",
      "detected_version": "v24.15.0",
      "required_version": "^v24.",
      "verify_command_label": "node -v",
      "beginner_message": "Node.js가 준비되었습니다.",
      "support_action": "없음",
      "evidence": {
        "exit_code": 0,
        "duration_ms": 42,
        "stdout_redacted": "v24.15.0",
        "stderr_redacted": ""
      },
      "links": ["https://nodejs.org/dist/latest-v24.x/"]
    }
  ],
  "execution_log": [],
  "resume_state": {
    "last_completed_step": "verify",
    "next_action": "open_lesson_project",
    "requires_restart": false
  },
  "redaction": {
    "applied": true,
    "rules_version": "0.1.0",
    "masked_fields": []
  }
}
```

## 5. Redaction rules

| 패턴 | 처리 |
| --- | --- |
| GitHub/Vercel token-like string | `[REDACTED_TOKEN]` |
| password-like key/value | `[REDACTED_PASSWORD]` |
| email address | `[REDACTED_EMAIL]` |
| macOS `/Users/<name>` | `/Users/[REDACTED_USER]` |
| Windows `C:\Users\<name>` | `C:\Users\[REDACTED_USER]` |
| OAuth/device code | `[REDACTED_CODE]` |

## 6. Instructor decision guide

| report summary | 강사 판단 |
| --- | --- |
| `ready_for_class` | 바로 수업 가능 |
| `needs_attention` | 수업 전 5~10분 지원 필요 |
| `blocked` | 조교/강사 직접 지원 필요, fallback 문서 사용 |
| `unsupported` | 장비/OS 교체 또는 대체 수업 경로 필요 |

## 7. Schema validation requirements

- JSON schema 문법이 유효해야 한다.
- example report가 schema 필수 필드를 모두 포함해야 한다.
- status enum은 recipe matrix와 동일해야 한다.
- report export 전 redaction이 `applied: true`여야 한다.
- `blocked` 또는 `unsupported` check는 `support_action`을 빈 문자열로 둘 수 없다.
