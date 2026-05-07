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

## Risk tier and action phase contract

| Risk tier | 의미 | Productized v1 behavior |
| --- | --- | --- |
| `safe` | 상태 확인만 수행 | 진단에서 자동 실행 가능 |
| `user_mediated` | 브라우저 로그인, 업데이트처럼 사용자가 승인해야 함 | 승인 큐 카드 표시 후 공식 흐름 열기 |
| `permission_prompt` | UAC, sudo, installer prompt 가능 | 권한 안내와 강사 도움 경로를 먼저 표시 |
| `blocked` | 앱이 실행하지 않는 흐름 | 실행하지 않고 handoff packet 생성 |

| Action phase | 의미 |
| --- | --- |
| `detect` | 상태 확인 |
| `install` | 사용자가 승인한 설치/준비 작업 |
| `external_flow` | 공식 브라우저/업데이트 흐름 |
| `manual_guidance` | 사용자가 직접 진행하는 안내 |
| `not_automated` | 자동 실행 금지 |

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
| Implemented action | `node.install.macos.pkg`: `latest-v24.x`의 macOS pkg를 다운로드하고 `osascript ... with administrator privileges`로 설치 |
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
| macOS | `npm install -g pnpm@latest` (`pnpm.install.macos.npm`) | `pnpm -v` | login shell PATH, npm global bin/PATH 확인 |
| Windows | `npm install -g pnpm@latest` (`pnpm.install.windows.npm`) | 새 PowerShell에서 `pnpm -v` | PATH/새 세션 안내 |

Gate note: package manager는 pnpm primary + npm available이지만, pnpm 설치 방식은 Node v24와 OS별 PATH 안정성을 기준으로 recipe dry-run 후 고정한다.

## 7. Vercel CLI / Vercel auth

| 플랫폼 | Install | Auth | Verify | 금지 |
| --- | --- | --- | --- | --- |
| macOS | `npm install -g vercel@latest` (`vercel.install.macos.npm`) | 최신 `vercel login` OAuth 2.0 Device Flow | `vercel whoami`; Terminal 창은 자동 종료하지 않음 | 금지: `vercel login --github`, `--gitlab`, `--bitbucket`, `--oob`, email 직접 로그인 플래그 |
| Windows | `npm install -g vercel@latest` (`vercel.install.windows.npm`) | 최신 `vercel login` OAuth 2.0 Device Flow | 새 PowerShell에서 `vercel whoami` | 금지: deprecated login flags |

Beginner copy: “Vercel 로그인은 브라우저/코드 확인 흐름으로 진행됩니다. 이 앱은 Vercel 비밀번호를 묻지 않습니다.”

## 8. Codex 앱

| 플랫폼 | Prerequisite repair | Install | Detect | Verify | Notes |
| --- | --- | --- | --- | --- | --- |
| Windows | `Microsoft.VCRedist.2015+.x64`, `Microsoft.EdgeWebView2Runtime` | Microsoft 공식 `Codex Installer.exe` 다운로드 링크 | 시작 메뉴 앱 목록에서 Codex 검색 | 설치 후 시작 메뉴 등록 확인 | 설치 화면과 권한 확인은 사용자가 직접 진행 |
| macOS | 현재 자동화 비대상 | 현재 자동화 비대상 | 앱 설치 여부는 수업 정책에 따라 별도 안내 | - | Windows 수업 흐름 우선 |

Codex Windows runtime repair:

| 항목 | Detect | Install/Repair | Verify | 오류 증상 |
| --- | --- | --- | --- | --- |
| VC++ Redistributable x64 | PowerShell registry `VisualStudio\14.0\VC\Runtimes\x64` 확인 | `winget install --id Microsoft.VCRedist.2015+.x64 -e` | registry Installed=1/version 확인 | `code=3221225781`, `0xC0000135`, app-server websocket closed |
| Edge WebView2 Runtime | EdgeUpdate WebView2 Runtime registry 확인 | `winget install --id Microsoft.EdgeWebView2Runtime -e` | WebView2 Runtime version 확인 | Store 앱/데스크톱 앱 실행 화면이 바로 닫힘 |

Beginner copy: “Codex 앱이 처음 실행되며 오류가 나도 괜찮습니다. 앱이 Windows 실행 런타임을 먼저 설치/복구한 뒤 Codex 설치를 이어갑니다.”

## 9. Supabase CLI

| 플랫폼 | Install | Detect | Auth | Verify | 금지 |
| --- | --- | --- | --- | --- | --- |
| macOS | 공식 문서의 Homebrew 방식 `brew install supabase/tap/supabase` | `supabase --version` | 공식 `supabase login` 흐름 | `supabase projects list` | 금지: `npm install -g supabase`, 앱이 access token 입력받기 |
| Windows | 공식 GitHub 릴리스의 `supabase_windows_amd64.tar.gz` 독립 실행 파일을 사용자 폴더에 설치하고 PATH 등록 | `supabase --version` | 공식 `supabase login` 흐름 | 새 PowerShell에서 `supabase projects list` | 금지: `npm install -g supabase`, 앱이 access token 저장 |

주의: Supabase 공식 문서 기준으로 npm 전역 설치는 지원되지 않는다. Node/npm 기반 수업에서는 필요할 때 `npx supabase ...` 또는 프로젝트 dev dependency 방식도 안내할 수 있지만, 이 앱의 전역 CLI 준비는 standalone/Homebrew 경로를 사용한다.

Beginner copy: “Supabase 가입/로그인은 공식 CLI 흐름에서 진행됩니다. 이 앱은 Supabase 토큰을 직접 묻거나 저장하지 않습니다.”

## 10. Homebrew / macOS prerequisites

| 항목 | Detect | Install/Repair | Verify | Notes |
| --- | --- | --- | --- | --- |
| macOS version | `sw_vers -productVersion` | OS 업데이트 안내 | Sonoma 14+ pass | Homebrew support 기준과 앱 지원 기준 분리 가능 |
| CLT | `xcode-select -p` | `xcode-select --install` 안내 | path exists | GUI prompt 가능 |
| Homebrew | `brew --version` | 공식 installer 안내/noninteractive 가능성 검토 | `brew --prefix` | Apple Silicon `/opt/homebrew`, Intel `/usr/local` |
| shellenv | shell profile/PATH 확인 | `eval "$(brew shellenv)"` 계열 profile 반영 안내 | 새 login shell command visibility | 변경 파일 report 포함 |

## 11. WinGet / Windows prerequisites

| 항목 | Detect | Install/Repair | Verify | Notes |
| --- | --- | --- | --- | --- |
| Windows version | PowerShell/CIM version | unsupported 안내 | Windows 10 2004+/Win11 | WSL minimum과 분리 가능 |
| WinGet | `winget --version` | App Installer/Store registration 안내 | minimum version gate | 첫 로그인/Store 등록 지연 가능 |
| WinGet source | `winget source list` | source reset/update 안내 | `winget search` 가능 | 조직 정책 차단 가능 |
| VC++ Redistributable x64 | registry 확인 | WinGet `Microsoft.VCRedist.2015+.x64` | registry Installed=1 | Codex `0xC0000135` 예방 |
| Edge WebView2 Runtime | registry 확인 | WinGet `Microsoft.EdgeWebView2Runtime` | WebView2 version | Windows 데스크톱 앱 런타임 |
| Admin ability | elevation probe | 관리자 실행 안내 | elevated command 가능 여부 | UAC consent 필요 |

## 12. WSL

| 정책 | Detect | Install/Repair | Verify | 상태 처리 |
| --- | --- | --- | --- | --- |
| optional | `wsl --status` | 설치하지 않음, 필요 시 안내 | 있으면 installed | 없으면 `optional_skipped` |
| required | Windows build + virtualization + `wsl --status` | 관리자 PowerShell `wsl --install`, 재부팅 후 이어하기 | `wsl --status`, distro first launch | missing/needs_restart/unsupported/blocked |

MVP default: optional. 커리큘럼에서 Linux shell/Docker가 필수인 회차만 required로 전환한다.

## 13. Tauri/WebView2/bootstrap support recipe

| 플랫폼 | Detect | Failure classes | Fallback |
| --- | --- | --- | --- |
| macOS | 앱 launch success, quarantine/Gatekeeper 상태 | unsigned/private build, Gatekeeper 차단 | 배포 문서, 스크린샷, fallback script bundle |
| Windows | installer launch, WebView2 availability/bootstrap | SmartScreen, AV, WebView2 bootstrap fail, org policy | WebView2 안내, signed build 계획, fallback script bundle |

## 14. Offline/cache fallback 필드

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

## 15. Forbidden recipe lint rules

- `vercel login --github` 사용 금지
- `npm install -g supabase` 사용 금지
- Codex `0xC0000135`/`3221225781` 복구 흐름에서 VC++ Redistributable과 WebView2 Runtime 확인 누락 금지
- 앱 자체 PAT/token/password prompt 금지
- raw shell string interpolation 금지
- allowlist 없는 arbitrary command 실행 금지
- checksum 없는 cached binary 사용 금지
- WSL optional을 failure로 처리 금지

## 16. Release-time freeze checklist

- [ ] Node latest-v24.x patch 확인
- [ ] Node pkg/MSI checksum 고정
- [ ] Homebrew `node@24` stable version 확인
- [ ] WinGet `OpenJS.NodeJS.LTS` manifest version 확인
- [ ] Git/GitHub CLI/Vercel CLI package ID 확인
- [ ] Codex 앱 다운로드 링크 확인
- [ ] Codex Windows runtime package ID 확인: `Microsoft.VCRedist.2015+.x64`, `Microsoft.EdgeWebView2Runtime`
- [ ] Supabase CLI 공식 설치 방식과 릴리스 asset 이름 확인
- [ ] Vercel login flow 최신성 확인
- [ ] WSL docs 최신성 확인
- [ ] offline/cache artifact checksum 확인
- [ ] beginner copy 업데이트
