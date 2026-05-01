# 위드지니 셋업

비개발자·초보 수강생이 수업 전에 개발 환경을 안전하게 점검할 수 있도록 만든 **Windows/macOS 데스크톱 앱**입니다. Node.js, npm, pnpm, Git, GitHub CLI, Vercel CLI, Windows/WSL 관련 상태를 한글 UI로 확인하고, 문제가 있으면 민감정보를 가린 리포트를 강사에게 전달할 수 있습니다.

> 단기 목표는 **Windows 수강생이 설치/진단에서 무너지지 않게 만드는 것**입니다. 앱은 비밀번호를 묻지 않고, 허용된 확인 명령만 실행합니다.

## 스크린샷

| 시작 화면 | Windows 진단 화면 |
| --- | --- |
| ![위드지니 셋업 시작 화면](docs/assets/screenshots/overview-macos.png) | ![위드지니 셋업 Windows 진단 화면](docs/assets/screenshots/overview-windows.png) |

## 핵심 기능

- **쉬운 한글 UI**: 초보자가 이해할 수 있는 진행 상태, 진단 결과, 다음 조치 안내
- **안전 진단**: allowlist 기반 확인 명령만 실행하고 비밀번호·토큰을 요청하지 않음
- **수업 준비 리포트**: 민감정보 redaction 후 JSON 리포트 복사
- **Windows 우선 검증**: Git, GitHub CLI, Vercel CLI, WSL 상태까지 Windows 기준으로 확인
- **업데이트 준비**: Tauri v2 updater, 서명된 release artifact, GitHub Actions release workflow 구성
- **위드지니 브랜딩**: Windows/macOS 앱 아이콘과 표시 이름 통일

## 설치 파일

공개 릴리즈는 GitHub Releases에서 받을 수 있습니다.

- Releases: https://github.com/NewTurn2017/withgenie-vibe-setup/releases

- Windows: `withgenie-setup_*_windows_*_setup.exe`
- macOS: `withgenie-setup_*_darwin_*.dmg`
- 업데이트 메타데이터: `latest.json` 및 `.sig`

## 개발 실행

```bash
npm ci
npm run tauri dev
```

## 검증

```bash
npm run check
npm run verify:docs
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## 릴리즈 빌드

업데이트 artifact를 만들려면 Tauri signing key가 필요합니다.

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat $HOME/.tauri/withgenie-vibe-setup.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri -- build
```

자세한 운영 절차는 [`docs/release/RELEASE.md`](docs/release/RELEASE.md)를 참고하세요.

## 보안 원칙

- 비밀번호, PAT, OAuth token을 앱에서 직접 입력받지 않습니다.
- GitHub 로그인은 `gh auth login`, Vercel 로그인은 `vercel login` 공식 흐름만 사용합니다.
- `vercel login --github` 같은 deprecated provider flag는 금지합니다.
- raw shell interpolation 없이 Rust allowlist 명령만 structured args로 실행합니다.
- 리포트에는 이메일, 홈 경로, 토큰 형태 문자열, OAuth device code를 가립니다.

## 프로젝트 구조

```text
src/                         React UI
src-tauri/                   Tauri/Rust 진단 runner, 메뉴, 번들 설정
schemas/                     health report JSON Schema
docs/                        수업 준비 매트릭스, 스키마, 릴리즈 운영 문서
.github/workflows/           CI 및 서명된 데스크톱 릴리즈 workflow
```

## 현재 범위

현재 버전은 **진단/리포트 중심 MVP**입니다. 실제 설치 자동화는 사용자 동의, 권한 상승, 공식 설치 경로 검증을 더한 뒤 단계적으로 추가합니다.
