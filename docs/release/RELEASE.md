# 릴리즈 운영 노트

Vibe Coding Setup은 Tauri v2 updater 흐름을 사용합니다.

## 서명 키

- 공개키: `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 포함되어 있습니다.
- 비공개키: 절대 저장소에 커밋하지 않습니다.
- GitHub Actions에는 `TAURI_SIGNING_PRIVATE_KEY` secret으로 비공개키 원문을 저장합니다.
- 비밀번호를 둔 키라면 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret도 함께 저장합니다.

로컬에서 키를 다시 만들 때:

```bash
npm run tauri -- signer generate -- -w ~/.tauri/withgenie-vibe-setup.key
```

## GitHub 설정

- 저장소: https://github.com/NewTurn2017/withgenie-vibe-setup
- `Actions > General > Workflow permissions`는 기본 `GITHUB_TOKEN`으로 release asset을 업로드할 수 있도록 `contents: write` 권한을 사용합니다. 현재 workflow의 job에 `permissions.contents: write`가 명시되어 있습니다.
- GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY`에는 로컬 `~/.tauri/withgenie-vibe-setup.key` 원문을 저장합니다.
- 비밀번호 없는 키라면 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`는 비워둘 수 있습니다.

## 릴리즈 만들기

1. `src-tauri/tauri.conf.json`과 `package.json`의 버전을 올립니다.
   - README의 직접 다운로드 링크 두 줄(macOS/Windows v버전)도 같은 버전으로 갱신합니다.
2. 검증을 통과시킵니다.

```bash
npm ci
npm run check
npm run verify:docs
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

3. 태그를 만들고 푸시합니다.

```bash
git tag v0.1.1
git push origin main --tags
```

GitHub Actions의 `Release desktop app` workflow가 Windows/macOS 번들과 updater용 `latest.json`, `.sig` 파일을 릴리즈에 올립니다. 초기 bootstrap 태그는 CI/Release workflow 검증 중 실패하거나 updater 메타데이터가 빠질 수 있으므로, workflow 수정 후에는 새 패치 버전 태그를 발행합니다.

## 로컬 릴리즈 빌드

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat $HOME/.tauri/withgenie-vibe-setup.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri -- build
```

Windows PowerShell:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\withgenie-vibe-setup.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npm run tauri -- build
```
