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

## macOS DMG 인증/공증

macOS 공개 배포용 DMG는 Developer ID 인증서로 서명하고 Apple Notary Service에 제출한 뒤, 최종 배포 파일인 `.dmg`에 notarization ticket을 staple합니다.

GitHub Actions에는 다음 secret이 필요합니다.

- `APPLE_CERTIFICATE`: Developer ID Application 인증서가 포함된 `.p12` 파일의 base64 원문
- `APPLE_CERTIFICATE_PASSWORD`: 해당 `.p12` export 비밀번호
- `APPLE_SIGNING_IDENTITY`: 예) `Developer ID Application: ... (TEAMID)`
- `APPLE_API_ISSUER`: App Store Connect API Issuer ID
- `APPLE_API_KEY`: App Store Connect API Key ID
- `APPLE_API_KEY_CONTENT`: `AuthKey_<KEYID>.p8` 파일 원문

현재 release workflow는 macOS job에서 다음 순서로 처리합니다.

1. Apple API key를 runner 임시 파일로 복원합니다.
2. `tauri-action`으로 `.app`을 Developer ID 서명/notarization/staple하고 DMG를 생성합니다.
3. 최종 배포 파일인 `.dmg`를 다시 `notarytool submit --wait`으로 제출합니다.
4. `xcrun stapler staple`과 `spctl -a -vvv -t install` 검증을 통과한 DMG만 release asset으로 다시 업로드합니다.
5. 모든 플랫폼 job이 끝난 뒤 draft release를 public release로 전환합니다.

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
export APPLE_SIGNING_IDENTITY="Developer ID Application: ..."
export APPLE_API_ISSUER="..."
export APPLE_API_KEY="..."
export APPLE_API_KEY_PATH="/path/to/AuthKey_${APPLE_API_KEY}.p8"
npm run tauri -- build
```

로컬에서 DMG까지 완전히 검증하려면 Tauri 빌드 후 최종 `.dmg`에 대해 다음을 추가로 실행합니다.

```bash
xcrun notarytool submit "path/to/Vibe Coding Setup_<version>_<arch>.dmg" \
  --key "$APPLE_API_KEY_PATH" \
  --key-id "$APPLE_API_KEY" \
  --issuer "$APPLE_API_ISSUER" \
  --wait
xcrun stapler staple "path/to/Vibe Coding Setup_<version>_<arch>.dmg"
xcrun stapler validate "path/to/Vibe Coding Setup_<version>_<arch>.dmg"
spctl -a -vvv -t install "path/to/Vibe Coding Setup_<version>_<arch>.dmg"
```

Windows PowerShell:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\withgenie-vibe-setup.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npm run tauri -- build
```
