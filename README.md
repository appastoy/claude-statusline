# claude-statusline

Claude Code용 PowerShell 커스텀 statusline 스크립트입니다.

## 표시 내용

```
🤖 Opus 4.7 1M H │ 🧠 ▁ 4% 37k (98%) │ 🔥 12k/m
⏳5h █ 90% │ 📅1w █ 98% │ ☕1h 5m
📂github\myproject │ 🌿main ✓ │ ↑1 │ 💰$7.78
```

- **모델 줄** — 모델명, 컨텍스트 크기(1M 이상일 때), effort 레벨(L/M/H/X/A/U), 컨텍스트 사용률 + 절대 토큰 수 + 캐시 적중률, 토큰 번 레이트(EMA) 및 컨텍스트 소진 예상 시간
- **한도 줄** — 5시간/1주 rate limit 남은 비율(임박 시 리셋 시각), 세션 경과 시간
- **작업 줄** — 현재 폴더, git 브랜치/상태(✚staged ●modified ?untracked, ↑↓ ahead/behind), 세션 비용

색상은 위험도에 따라 초록 → 노랑 → 주황 → 빨강으로 변합니다 (컨텍스트는 높을수록, 한도는 낮을수록 경고).

## 설치

1. `statusline.ps1`을 원하는 위치에 저장합니다 (예: `~/.claude/statusline/statusline.ps1`).
2. `~/.claude/settings.json`에 다음을 추가합니다:

```json
{
  "statusLine": {
    "type": "command",
    "command": "pwsh -NoProfile -File \"C:\\Users\\<사용자>\\.claude\\statusline\\statusline.ps1\""
  }
}
```

PowerShell 7(pwsh)이 필요합니다. git이 설치되어 있으면 git 정보도 표시됩니다.

## 파일

| 파일 | 설명 |
|---|---|
| `statusline.ps1` | 실제 statusline 스크립트 |
| `statusline-example.json` | Claude Code가 stdin으로 넘겨주는 입력 payload 예시 |
| `statusline-mockup.txt` | 레이아웃/색상 스펙 목업 |

스크립트는 실행 시 같은 폴더에 `burn_state.json`(번 레이트 EMA 상태 파일)을 생성합니다.
