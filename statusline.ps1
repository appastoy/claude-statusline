# Claude Code statusline — statusline-mockup.txt(v0.1) 기준 재구성
#
# 배치
#   1줄: <폴더>  │  <git>  │  <ahead/behind>  │  <변경라인>  │  <세션시간>  │  <비용>
#   2줄: <모델>  │  ctx <문맥%>  │  5h <남은%>  │  1w <남은%>
#
# 색 기준
#   문맥(ctx) 높을수록 경고:  0-49 초록 / 50-74 노랑 / 75-89 주황 / 90-100 빨강
#   한도(5h/1w) 낮을수록 경고: 50-100 초록 / 25-49 노랑 / 10-24 주황 / 0-9 빨강
#   남은 비율 25% 미만일 때만 리셋시각 표시 (5h→HH:mm, 1w→{n}d HH:mm)

$ErrorActionPreference = 'SilentlyContinue'

# 최종 출력을 UTF-8 바이트로 표준출력에 직접 쓰는 헬퍼.
# [Console]::OutputEncoding 을 건드리면 출력이 파이프로 리다이렉트된 상태에서
# "핸들이 유효하지 않음" 예외가 나 스크립트가 통째로 죽으므로 사용하지 않는다.
function Emit([string]$text) {
  $bytes  = [System.Text.Encoding]::UTF8.GetBytes($text)
  $stdout = [Console]::OpenStandardOutput()
  $stdout.Write($bytes, 0, $bytes.Length)
  $stdout.Flush()
}

# ---- stdin (Claude Code가 매 렌더마다 JSON payload를 파이프로 전달) ----
$raw = [Console]::In.ReadToEnd()
try { $i = $raw | ConvertFrom-Json } catch { return }
if (-not $i) { return }

# ---- ANSI 색 헬퍼 ----
$e   = [char]27
$RST = "$e[0m"
function Paint([string]$text, [string]$code) { "$e[${code}m$text$RST" }
$GREEN  = '38;5;71'
$YELLOW = '38;5;179'
$ORANGE = '38;5;208'
$RED    = '38;5;167'
$MUTED  = '38;5;240'
$CYAN   = '38;5;80'
$BLUE   = '38;5;75'
$PURPLE = '38;5;140'
$WHITE  = '38;5;255'
$SKY    = '38;5;110'
$PEACH  = '38;5;215'
$GOLD   = '38;5;220'
$MAGENTA = '38;5;176'  # 오키드 — 채도 낮은 마젠타

$SEP = Paint ' │ ' $MUTED

function CtxColor([double]$used) {
  if     ($used -lt 50) { $GREEN }
  elseif ($used -lt 75) { $YELLOW }
  elseif ($used -lt 90) { $ORANGE }
  else                  { $RED }
}
function LimitColor([double]$rem) {
  if     ($rem -ge 50) { $GREEN }
  elseif ($rem -ge 25) { $YELLOW }
  elseif ($rem -ge 10) { $ORANGE }
  else                 { $RED }
}
function BarChar([double]$pct) {
  $bars = '▁','▂','▃','▄','▅','▆','▇','█'
  $idx  = [math]::Min(7, [math]::Max(0, [int][math]::Floor($pct / 100 * 8)))
  return $bars[$idx]
}

# 1줄: <폴더>  │  <git branch>
# 2줄: <모델>  │  ctx %
# 3줄: 5h %  │  1w %
$seg1 = New-Object System.Collections.Generic.List[string]
$seg2 = New-Object System.Collections.Generic.List[string]
$seg3 = New-Object System.Collections.Generic.List[string]

# 모델 (1줄) — 컨텍스트 크기 표시(실제 최대 컨텍스트 ≥ 1M일 때만), effort 약자 (L/M/H/X/A/U)
$rawName  = $i.model.display_name
$model    = ($rawName -replace '\s*\(.*\)\s*$', '').Trim()
$cwMaxRaw = [double]$i.context_window.context_window_size
$ctxLabel = if ($cwMaxRaw -ge 1000000) {
  $m = $cwMaxRaw / 1000000
  if ($m -eq [math]::Floor($m)) { "{0}M" -f [int]$m } else { "{0:0.0}M" -f $m }
} else { '' }
if ($model) {
  $mtxt = "🤖" + (Paint $model $ORANGE)
  if ($ctxLabel) { $mtxt += (Paint " $ctxLabel" $MAGENTA) }
  $lvl = $i.effort.level
  $effColor = @{
    low       = '38;5;240'
    medium    = '38;5;109'
    high      = '38;5;71'
    xhigh     = '38;5;179'
    max       = '38;5;208'
    ultracode = '1;38;5;205'
  }
  $effShort = @{ low='L'; medium='M'; high='H'; xhigh='X'; max='A'; ultracode='U' }
  if ($lvl -and $effColor.ContainsKey($lvl)) {
    $mtxt += ' ' + (Paint $effShort[$lvl] $effColor[$lvl])
  }
  $seg2.Add($mtxt)
}

# 폴더 (1줄) — 부모\현재 2깊이. 부모가 드라이브 루트면 드라이브까지 (예: D:\github)
$cur = $i.workspace.current_dir; if (-not $cur) { $cur = $i.cwd }
if ($cur) {
  $curN  = $cur.TrimEnd('\','/')
  $parts = $curN -split '[\\/]+'
  if ($parts.Count -ge 2) {
    $folder = ($parts[-2, -1]) -join '\'
  } else {
    $folder = $parts[-1]
  }
  # 드라이브 루트만 남으면 백슬래시 보정 (예: "D:" -> "D:\")
  if ($folder -match '^[A-Za-z]:$') { $folder += '\' }
  $seg1.Add((Paint "📂$folder" $BLUE))
}

# 토큰 수를 k/M로 압축 (0->0k, 123000->123k, 200000->200k, 1000000->1M, 1500000->1.5M)
function FmtTok([double]$n) {
  if ($n -ge 1000000) {
    $m = $n / 1000000
    if ($m -eq [math]::Floor($m)) { return ("{0}M" -f [int]$m) }
    return ("{0:0.0}M" -f $m)
  }
  return ("{0}k" -f [int][math]::Round($n / 1000))
}

# ctx (2줄) — "ctx 4% (37k/1M)" — 값이 없거나 0이어도 항상 표시
$cw = $i.context_window
$ctxUsed = if ($null -ne $cw.used_percentage) { [double]$cw.used_percentage } else { 0 }
$cwMax = [double]$cw.context_window_size
if ($null -ne $cw.total_input_tokens) {
  $cwCur = [double]$cw.total_input_tokens
} elseif ($cw.current_usage) {
  $u = $cw.current_usage
  $cwCur = [double]($u.input_tokens + $u.cache_creation_input_tokens + $u.cache_read_input_tokens)
} else {
  $cwCur = 0
}
$maxTxt = if ($cwMax -gt 0) { FmtTok $cwMax } else { '?' }
$ctxColor = CtxColor $ctxUsed
# 캐시 적중률 — ctx 우측에 "(100%)" 형태로 병기, cache_read > 0 일 때만
$cu = $cw.current_usage
$hitTxt = ''
if ($cu -and [double]$cu.cache_read_input_tokens -gt 0) {
  $cTotal = [double]$cu.input_tokens + [double]$cu.cache_creation_input_tokens + [double]$cu.cache_read_input_tokens
  $hitPct = [int]($cu.cache_read_input_tokens / $cTotal * 100)
  $hitColor = if ($hitPct -ge 75) { $GREEN } elseif ($hitPct -ge 40) { $YELLOW } else { $MUTED }
  $hitTxt = " " + (Paint "(${hitPct}%)" $hitColor)
}
$seg2.Add("🧠 " + (Paint ("$(BarChar $ctxUsed) $([int]$ctxUsed)%") $ctxColor) + " " + (Paint (FmtTok $cwCur) $WHITE) + $hitTxt)

# 번 레이트 (2줄) — 상태 파일 EMA: 토큰 변화 시점 = 턴 완료 시점
$burnStateFile = Join-Path $PSScriptRoot 'burn_state.json'
$bs      = if (Test-Path $burnStateFile) { try { Get-Content $burnStateFile -Raw | ConvertFrom-Json } catch { $null } } else { $null }
$nowMs   = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$emaRate = if ($bs -and $bs.ema_rate) { [double]$bs.ema_rate } else { 0 }

if ($cwCur -gt 0) {
  $prevTok = if ($bs) { [double]$bs.last_tokens } else { 0 }
  $prevTs  = if ($bs) { [double]$bs.last_ts }     else { $nowMs }
  $dTok = $cwCur - $prevTok
  $dMin = ($nowMs - $prevTs) / 60000

  if ($dTok -gt 0 -and $dMin -gt 0.1) {
    # 토큰 증가 = 턴 완료: instant rate → EMA 업데이트
    $instant = $dTok / $dMin
    $alpha   = 0.35
    $emaRate = if ($emaRate -gt 0) { $alpha * $instant + (1 - $alpha) * $emaRate } else { $instant }
    @{ last_tokens = $cwCur; last_ts = $nowMs; ema_rate = $emaRate } | ConvertTo-Json | Set-Content $burnStateFile
  } elseif ($prevTok -eq 0) {
    # 첫 렌더: 기준점 초기화만
    @{ last_tokens = $cwCur; last_ts = $nowMs; ema_rate = 0 } | ConvertTo-Json | Set-Content $burnStateFile
  } elseif ($cwCur -lt $prevTok * 0.5) {
    # 컨텍스트 압축 감지: 타임스탬프 리셋, ema_rate 유지
    @{ last_tokens = $cwCur; last_ts = $nowMs; ema_rate = $emaRate } | ConvertTo-Json | Set-Content $burnStateFile
  }
  # dTok == 0 (idle) 또는 dMin <= 0.1 (너무 짧은 간격): 상태 파일 그대로
}

if ($emaRate -gt 0) {
  $rateK   = $emaRate / 1000
  $rateTxt = if ($rateK -lt 1) { "<1k/m" } else { "{0:0}k/m" -f $rateK }
  $burnSeg = "🔥 " + (Paint $rateTxt $MUTED)
  if ($ctxUsed -ge 60 -and $cwMax -gt 0) {
    $minsLeft = [int](($cwMax - $cwCur) / $emaRate)
    $tColor   = if ($minsLeft -lt 10) { $RED } elseif ($minsLeft -lt 30) { $ORANGE } elseif ($minsLeft -lt 60) { $YELLOW } else { $MUTED }
    $remTxt   = if ($minsLeft -ge 60) { "{0}h{1:D2}m" -f [int]($minsLeft/60), ($minsLeft%60) } else { "${minsLeft}m" }
    $burnSeg += " " + (Paint "~$remTxt" $tColor)
  }
  $seg2.Add($burnSeg)
} else {
  $seg2.Add((Paint "🔥 --" $MUTED))
}

# 한도 (resets_at: unix epoch 초)
function FmtLimit([string]$label, $node, [string]$kind) {
  if ($null -eq $node -or $null -eq $node.used_percentage) { return $null }
  $rem = 100 - [double]$node.used_percentage
  $remI = [int][math]::Round($rem)
  $lc = LimitColor $rem
  $result = "$label " + (Paint "$(BarChar $rem) ${remI}%" $lc)
  if ($node.resets_at) {
    $reset = [DateTimeOffset]::FromUnixTimeSeconds([long]$node.resets_at).LocalDateTime
    if ($kind -eq '5h') {
      $result += Paint (' ' + $reset.ToString('HH:mm')) $MUTED
    } else {
      $days = ($reset.Date - (Get-Date).Date).Days
      $result += Paint (' ' + ("{0}d {1}" -f $days, $reset.ToString('HH:mm'))) $MUTED
    }
  }
  return $result
}
$rl = $i.rate_limits
if ($rl) {
  $s5 = FmtLimit '⏳5h' $rl.five_hour '5h';  if ($s5) { $seg3.Add($s5) }
  $s7 = FmtLimit '📅1w' $rl.seven_day '1w';  if ($s7) { $seg3.Add($s7) }
}

# git (1줄) — porcelain=v2 --branch 1회 호출로 브랜치/ahead-behind/변경수 전부 획득
$gitDir = $cur
$branch = $null; $ahead = 0; $behind = 0
$staged = 0; $modified = 0; $untracked = 0
$inRepo = $false
if ($gitDir) {
  $porc = & git -C $gitDir --no-optional-locks status --porcelain=v2 --branch 2>$null
  if ($LASTEXITCODE -eq 0 -and $porc) {
    $inRepo = $true
    foreach ($ln in $porc) {
      if ($ln.StartsWith('# branch.head ')) {
        $branch = $ln.Substring(14).Trim()
      } elseif ($ln.StartsWith('# branch.ab ')) {
        $ab = $ln.Substring(12).Trim() -split '\s+'
        foreach ($t in $ab) {
          if     ($t.StartsWith('+')) { $ahead  = [int]$t.Substring(1) }
          elseif ($t.StartsWith('-')) { $behind = [int]$t.Substring(1) }
        }
      } elseif ($ln.StartsWith('1 ') -or $ln.StartsWith('2 ')) {
        $xy = $ln.Substring(2,2)
        if ($xy[0] -ne '.') { $staged++ }
        if ($xy[1] -ne '.') { $modified++ }
      } elseif ($ln.StartsWith('? ')) {
        $untracked++
      }
    }
  }
}
if ($inRepo) {
  if (-not $branch -or $branch -eq '(detached)') { $branch = 'detached' }
  $gitTxt = "🌿" + (Paint $branch $PURPLE)
  if ($staged -eq 0 -and $modified -eq 0 -and $untracked -eq 0) {
    $gitTxt += ' ' + (Paint '✓' $GREEN)
  } else {
    $marks = @()
    if ($staged)    { $marks += (Paint "✚$staged" $CYAN) }
    if ($modified)  { $marks += (Paint "●$modified" $YELLOW) }
    if ($untracked) { $marks += (Paint "?$untracked" $MUTED) }
    $gitTxt += ' ' + ($marks -join ' ')
  }
  $seg1.Add($gitTxt)

  if ($ahead -or $behind) {
    $ab2 = @()
    if ($ahead)  { $ab2 += (Paint "↑$ahead" $SKY) }
    if ($behind) { $ab2 += (Paint "↓$behind" $ORANGE) }
    $seg1.Add(($ab2 -join ' '))
  }
} else {
  $seg1.Add((Paint '🌿no git' $MUTED))
}

# 세션 시간 — 한도 줄(표시 2번째 줄) 마지막에 표시
$durMs = [double]$i.cost.total_duration_ms
if ($durMs -gt 0) {
  $min = [int][math]::Floor($durMs / 60000)
  $seg3.Add("☕" + (Paint ("{0}h {1}m" -f [int][math]::Floor($min / 60), ($min % 60)) $PEACH))
}

# 비용 (1줄 맨 오른쪽)
$cost = $i.cost.total_cost_usd
if ($null -ne $cost) {
  $seg1.Add((Paint ('💰${0:N2}' -f [double]$cost) $GOLD))
}

# ================= 출력 =================
$line1 = ($seg1 -join $SEP)
$line2 = ($seg2 -join $SEP)
$line3 = ($seg3 -join $SEP)
if ($line3) { Emit "$line2`n$line3`n$line1" } elseif ($line2) { Emit "$line2`n$line1" } else { Emit $line1 }
