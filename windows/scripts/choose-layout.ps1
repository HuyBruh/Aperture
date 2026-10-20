# Pick a layout / theme / style. Writes trofeo.conf - the display updates within ~1 second.
$conf = Join-Path (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)) 'trofeo.conf'
$layouts = @(
  @('classic',   'Classic   - rounded cards (the original)                          [landscape]'),
  @('cockpit',   'Cockpit   - twin automotive gauges, clock in the middle           [landscape]'),
  @('focus',     'Focus     - giant digits, minimal                                 [landscape]'),
  @('bento',     'Bento     - saturated colour tiles, Apple-keynote style           [landscape]'),
  @('terminal',  'Terminal  - green console with ASCII bars                         [landscape]'),
  @('chamfer',   'Chamfer   - angular HUD panels, slanted numerals                  [landscape]'),
  @('rings',     'Rings     - nested activity rings for CPU / GPU / memory          [landscape]'),
  @('dots',      'Dots      - LED dot-matrix numerals                               [landscape]'),
  @('lock',      'Lock      - phone lock screen, frosted glass over an aurora       [portrait]'),
  @('equalizer', 'Equalizer - hi-fi LED level meters                                [portrait]'),
  @('receipt',   'Receipt   - paper receipt, barcode, halftone album art            [portrait]'),
  @('synthwave', 'Synthwave - retro sunset and neon grid                            [portrait]'),
  @('blueprint', 'Blueprint - technical drawing with a title block                  [portrait]'),
  @('all',       'Cycle through all LANDSCAPE layouts'),
  @('portrait',  'Cycle through the 5 PORTRAIT layouts')
)
$themes = @(
  @('nzxt',     'NZXT     - violet / cyan / pink (default)'),
  @('graphite', 'Graphite - monochrome silver, very minimal'),
  @('crimson',  'Crimson  - red / orange gaming look'),
  @('arctic',   'Arctic   - ice blue / teal on deep navy'),
  @('ember',    'Ember    - orange / amber'),
  @('neon',     'Neon     - pink / cyan / yellow cyberpunk'),
  @('all',      'Cycle through all themes')
)
$styles = @(
  @('cards', 'Cards - rounded cards (default)'),
  @('glass', 'Glass - softly tinted glass cards'),
  @('flat',  'Flat  - no cards, thin rules only')
)
function Set-Key($text, $key, $value) {
  if ($text -match "(?m)^$key\s*=[^\r\n]*") { return [regex]::Replace($text, "(?m)^$key\s*=[^\r\n]*", "$key = $value") }
  return $text.TrimEnd() + "`r`n$key = $value`r`n"
}
function Ask($title, $items) {
  Write-Host ''
  Write-Host "  $title" -ForegroundColor Cyan
  for ($i = 0; $i -lt $items.Count; $i++) { Write-Host ("   {0,2}. {1}" -f ($i + 1), $items[$i][1]) }
  $a = Read-Host '  Enter a number (Enter = keep current)'
  if ($a -match '^\d+$' -and [int]$a -ge 1 -and [int]$a -le $items.Count) { return $items[[int]$a - 1][0] }
  return $null
}
$l = Ask 'CHOOSE A LAYOUT' $layouts
$t = Ask 'CHOOSE A COLOR THEME' $themes
$s = Ask 'CHOOSE A CARD STYLE (classic / cockpit / focus only)' $styles
$text = Get-Content $conf -Raw
if ($l) {
  $text = Set-Key $text 'pro_layout' $l
  if (@('lock', 'equalizer', 'receipt', 'synthwave', 'blueprint', 'portrait') -contains $l) {
    Write-Host ''
    Write-Host '  Portrait layouts need the screen mounted vertically. Rotation direction:' -ForegroundColor Cyan
    Write-Host '    1. clockwise (cw)'
    Write-Host '    2. counter-clockwise (ccw)  - pick this if the picture is upside-down'
    $r = Read-Host '  Enter a number (Enter = keep current)'
    if ($r -eq '1') { $text = Set-Key $text 'pro_rotate' 'cw' }
    if ($r -eq '2') { $text = Set-Key $text 'pro_rotate' 'ccw' }
  }
}
if ($t) { $text = Set-Key $text 'pro_theme' $t }
if ($s) { $text = Set-Key $text 'pro_style' $s }
[IO.File]::WriteAllText($conf, $text, (New-Object Text.UTF8Encoding $false))
Write-Host ''
Write-Host '  Saved! The display updates within a second.' -ForegroundColor Green
Start-Sleep -Seconds 2
