# Recipe Matrix: 원클릭 개발 환경 GUI 설치기

- 작성일: 2026-05-01
- 상태: Draft for gate review
- 관련 PRD: `.omx/plans/prd-one-click-dev-setup-installer.md`
- 관련 test spec: `.omx/plans/test-spec-one-click-dev-setup-installer.md`
- 원칙: 공식 경로 우선, idempotent detect/install/repair/verify, credential 비수집, release-time exact version/checksum freeze

## 1. Recipe status enum

| 상태 | 의미 |
| --- | --- |
| `installed` | 설치/버전/PATH/auth 검증 통과 |
| `missing` | 설치되지 않음 |
| `needs_repair` | 설치됨이나 버전/PATH/auth가 요구사항 미달 |
| `needs_restart` | 재부팅/새 shell/재로그인 필요 |
| `optional_skipped` | 선택 항목이라 건너뜀 |
| `unsupported` | OS/정책/버전상 지원 불가 |
| `blocked` | 권한/네트워크/정책/다운로드 차단 |

## 2. 공통 recipe 필드

각 recipe는 구현 전 아래 필드를 채워야 한다.

| 필드 | 설명 |
| --- | --- |
| `id` | stable identifier |
| `platform` | `macos`, `windows`, `common` |
| `required_for_mvp` | 필수 여부 |
| `detect` | 상태 확인 command/logic |
| `install_primary` | 기본 설치 경로 |
| `install_fallback` | fallback 설치 경로 |
| `repair` | PATH/auth/version 복구 경로 |
| `verify` | 최종 검증 command/logic |
| `requires_admin` | sudo/UAC/Admin PowerShell 여부 |
| `requires_browser` | OAuth/browser flow 여부 |
| `requires_restart` | reboot/new shell 여부 |
| `offline_cache` | 사전 다운로드/캐시 가능성, checksum, 만료 정책 |
| `rollback` | 실패/부분 설치 시 되돌림 또는 정리 정책 |
| `redaction` | 로그에서 숨겨야 할 패턴 |
| `beginner_copy` | 초보자용 설명 문구 |

## 3. Node.js v24 LTS(Krypton)

### macOS

| 필드 | 값 |
| --- | --- |
| ID | `node.macos.v24` |
| Primary | Node.js 공식 `latest-v24.x`의 `node-v24.*.pkg` |
| Primary source | https://nodejs.org/dist/latest-v24.x/ |
| Checksum | `SHASUMS256.txt`에서 release-time freeze |
| Fallback | Homebrew `node@24` |
| Fallback source | https://formulae.brew.sh/formula/node@24 |
| Detect | `node -v`, `npm -v`, package receipt 또는 PATH 확인 |
| Verify | 새 login shell에서 `node -v`가 `^v24\.`이고 `npm -v` 성공 |
| Repair | PATH 미반영 시 shell profile 업데이트 또는 Homebrew `node@24` keg-only PATH 안내 |
| Admin | 공식 pkg는 OS installer 권한 프롬프트 가능, Homebrew는 설치 상태에 따라 sudo/CLT 필요 가능 |
| Offline/cache | pkg와 `SHASUMS256.txt` 캐시 가능. 캐시 만료는 release checklist에서 latest-v24.x 비교 |
| Rollback | 자동 삭제는 MVP 비목표. 실패 시 report에 partial install 기록 |

### Windows

| 필드 | 값 |
| --- | --- |
| ID | `node.windows.v24` |
| Primary | WinGet `OpenJS.NodeJS.LTS`, 단 `winget show --id OpenJS.NodeJS.LTS -e --source winget` 결과가 v24.x일 때만 |
| Primary source | https://github.com/microsoft/winget-pkgs/tree/master/manifests/o/OpenJS/NodeJS/LTS |
| Fallback | Node.js 공식 `latest-v24.x` MSI: `node-v24.*-x64.msi` 또는 `node-v24.*-arm64.msi` |
| Fallback source | https://nodejs.org/dist/latest-v24.x/ |
| Checksum | `SHASUMS256.txt`에서 release-time freeze |
| Detect | `node -v`, `npm -v`, WinGet package state, PATH 확인 |
| Verify | 새 PowerShell에서 `node -v`가 `^v24\.`이고 `npm -v` 성공 |
| Repair | PATH 미반영 시 새 PowerShell/재로그인/재부팅 안내 |
| Admin | WinGet/MSI 설치 시 UAC 가능 |
| Offline/cache | MSI와 checksum 캐시 가능. WinGet 실패 시 MSI fallback |
| Rollback | 자동 uninstall은 MVP 비목표. 부분 설치 상태 report 기록 |

## 4. Git

| 플랫폼 | Primary | Detect | Verify | Repair/Fallback |
| --- | --- | --- | --- | --- |
| macOS | Homebrew `git` 또는 CLT 제공 Git | `git --version` | 새 login shell에서 `git --version` 성공 | CLT 설치 안내, Homebrew 설치 |
| Windows | WinGet `Git.Git` | `git --version` | 새 PowerShell에서 `git --version` 성공 | Git for Windows 공식 installer fallback |

주의: macOS CLT 설치는 GUI/system prompt가 발생할 수 있으므로 beginner copy를 별도로 둔다.

## 5. GitHub CLI / GitHub auth

| 플랫폼 | Install | Auth | Verify | 금지 |
| --- | --- | --- | --- | --- |
| macOS | Homebrew `gh` | `gh auth login` 공식 browser/device flow | `gh auth status` | 앱이 PAT/token 입력받기 금지 |
| Windows | WinGet `GitHub.cli` | `gh auth login` 공식 browser/device flow | `gh auth status` | 앱이 PAT/token 저장 금지 |

Beginner copy: “GitHub 로그인은 브라우저에서 진행됩니다. 이 앱은 GitHub 비밀번호를 묻지 않습니다.”

## 6. pnpm

| 플랫폼 | Primary | Verify | Repair |
| --- | --- | --- | --- |
| macOS | Corepack 또는 npm 기반 설치 중 PRD gate에서 결정. MVP 기본은 `npm i -g pnpm` 또는 `corepack enable pnpm` 중 recipe dry-run으로 결정 | `pnpm -v` | global bin/PATH 확인 |
| Windows | Corepack 또는 npm 기반 설치 중 PRD gate에서 결정. MVP 기본은 `npm i -g pnpm` 또는 `corepack enable pnpm` 중 recipe dry-run으로 결정 | 새 PowerShell에서 `pnpm -v` | PATH/새 세션 안내 |

Gate note: package manager는 pnpm primary + npm available이지만, pnpm 설치 방식은 Node v24와 OS별 PATH 안정성을 기준으로 recipe dry-run 후 고정한다.

## 7. Vercel CLI / Vercel auth

| 플랫폼 | Install | Auth | Verify | 금지 |
| --- | --- | --- | --- | --- |
| macOS | `pnpm i -g vercel` 또는 npm fallback | 최신 `vercel login` OAuth 2.0 Device Flow | `vercel whoami` | 금지: `vercel login --github`, `--gitlab`, `--bitbucket`, `--oob`, email 직접 로그인 플래그 |
| Windows | `pnpm i -g vercel` 또는 npm fallback | 최신 `vercel login` OAuth 2.0 Device Flow | 새 PowerShell에서 `vercel whoami` | 금지: deprecated login flags |

Beginner copy: “Vercel 로그인은 브라우저/코드 확인 흐름으로 진행됩니다. 이 앱은 Vercel 비밀번호를 묻지 않습니다.”

## 8. Homebrew / macOS prerequisites

| 항목 | Detect | Install/Repair | Verify | Notes |
| --- | --- | --- | --- | --- |
| macOS version | `sw_vers -productVersion` | OS 업데이트 안내 | Sonoma 14+ pass | Homebrew support 기준과 앱 지원 기준 분리 가능 |
| CLT | `xcode-select -p` | `xcode-select --install` 안내 | path exists | GUI prompt 가능 |
| Homebrew | `brew --version` | 공식 installer 안내/noninteractive 가능성 검토 | `brew --prefix` | Apple Silicon `/opt/homebrew`, Intel `/usr/local` |
| shellenv | shell profile/PATH 확인 | `eval "$(brew shellenv)"` 계열 profile 반영 안내 | 새 login shell command visibility | 변경 파일 report 포함 |

## 9. WinGet / Windows prerequisites

| 항목 | Detect | Install/Repair | Verify | Notes |
| --- | --- | --- | --- | --- |
| Windows version | PowerShell/CIM version | unsupported 안내 | Windows 10 2004+/Win11 | WSL minimum과 분리 가능 |
| WinGet | `winget --version` | App Installer/Store registration 안내 | minimum version gate | 첫 로그인/Store 등록 지연 가능 |
| WinGet source | `winget source list` | source reset/update 안내 | `winget search` 가능 | 조직 정책 차단 가능 |
| Admin ability | elevation probe | 관리자 실행 안내 | elevated command 가능 여부 | UAC consent 필요 |

## 10. WSL

| 정책 | Detect | Install/Repair | Verify | 상태 처리 |
| --- | --- | --- | --- | --- |
| optional | `wsl --status` | 설치하지 않음, 필요 시 안내 | 있으면 installed | 없으면 `optional_skipped` |
| required | Windows build + virtualization + `wsl --status` | 관리자 PowerShell `wsl --install`, 재부팅 후 이어하기 | `wsl --status`, distro first launch | missing/needs_restart/unsupported/blocked |

MVP default: optional. 커리큘럼에서 Linux shell/Docker가 필수인 회차만 required로 전환한다.

## 11. Tauri/WebView2/bootstrap support recipe

| 플랫폼 | Detect | Failure classes | Fallback |
| --- | --- | --- | --- |
| macOS | 앱 launch success, quarantine/Gatekeeper 상태 | unsigned/private build, Gatekeeper 차단 | 배포 문서, 스크린샷, fallback script bundle |
| Windows | installer launch, WebView2 availability/bootstrap | SmartScreen, AV, WebView2 bootstrap fail, org policy | WebView2 안내, signed build 계획, fallback script bundle |

## 12. Offline/cache fallback 필드

각 다운로드 recipe는 release-time에 아래를 채운다.

```yaml
offline_cache:
  cacheable: true
  artifact_name: "node-v24.15.0.pkg"
  source_url: "https://nodejs.org/dist/latest-v24.x/..."
  checksum_algorithm: "sha256"
  checksum_source: "SHASUMS256.txt"
  cache_location: "support-bundle/cache/"
  expires_when: "latest-v24.x patch changes or release checklist refresh"
  fallback_if_cache_missing: "online download with checksum verification"
```

## 13. Forbidden recipe lint rules

- `vercel login --github` 사용 금지
- 앱 자체 PAT/token/password prompt 금지
- raw shell string interpolation 금지
- allowlist 없는 arbitrary command 실행 금지
- checksum 없는 cached binary 사용 금지
- WSL optional을 failure로 처리 금지

## 14. Release-time freeze checklist

- [ ] Node latest-v24.x patch 확인
- [ ] Node pkg/MSI checksum 고정
- [ ] Homebrew `node@24` stable version 확인
- [ ] WinGet `OpenJS.NodeJS.LTS` manifest version 확인
- [ ] Git/GitHub CLI/Vercel CLI package ID 확인
- [ ] Vercel login flow 최신성 확인
- [ ] WSL docs 최신성 확인
- [ ] offline/cache artifact checksum 확인
- [ ] beginner copy 업데이트
