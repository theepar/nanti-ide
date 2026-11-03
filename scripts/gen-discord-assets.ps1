# Draws the language badges that the Discord status shows next to the logo.
# Upload every PNG in the output folder at: Discord Developer Portal > your app > Rich Presence > Art Assets.
# The file name (without .png) becomes the asset key, so the names below must not change.
param([string]$Out = (Join-Path $PSScriptRoot '..\target\discord-assets'))

Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force $Out | Out-Null

# key, text on the badge, background colour
$badges = @(
    @('rust', 'RS', '#de7f57'), @('go', 'GO', '#58c4dd'), @('dart', 'DT', '#4ec9b0'),
    @('javascript', 'JS', '#e5c07b'), @('typescript', 'TS', '#4f9fe8'), @('react', 'JSX', '#61dafb'),
    @('python', 'PY', '#4b8bbe'), @('html', '<>', '#e37933'), @('css', '#', '#9b7fe8'),
    @('json', '{}', '#cbb26b'), @('markdown', 'MD', '#6fb1e8'), @('powershell', '>_', '#5391fe'),
    @('shell', '>_', '#8bc34a'), @('php', 'PHP', '#8993be'), @('java', 'J', '#e76f00'),
    @('kotlin', 'KT', '#a97bff'), @('c', 'C', '#519aba'), @('cpp', 'C++', '#00599c'),
    @('csharp', 'C#', '#6fb26f'), @('ruby', 'RB', '#cc3e44'), @('swift', 'SW', '#f05138'),
    @('lua', 'LUA', '#51a0cf'), @('sql', 'SQL', '#dad8d8'), @('yaml', 'YML', '#9aa5b1'),
    @('toml', 'TM', '#9aa5b1'), @('vue', 'VUE', '#42b883'), @('svelte', 'SV', '#ff3e00')
)

function New-RoundedPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    return $p
}

foreach ($b in $badges) {
    $key, $text, $hex = $b
    $bmp = New-Object System.Drawing.Bitmap 1024, 1024
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.TextRenderingHint = 'AntiAliasGridFit'
    $g.Clear([System.Drawing.Color]::Transparent)
    $color = [System.Drawing.ColorTranslator]::FromHtml($hex)
    $path = New-RoundedPath 64 64 896 896 200
    $g.FillPath((New-Object System.Drawing.SolidBrush $color), $path)

    # Shrink the font until the text fits the badge.
    $size = 420
    do {
        $font = New-Object System.Drawing.Font('Segoe UI', $size, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
        $m = $g.MeasureString($text, $font)
        $size -= 20
    } while ($m.Width -gt 720 -and $size -gt 100)
    $x = (1024 - $m.Width) / 2
    $y = (1024 - $m.Height) / 2
    $lum = (0.299 * $color.R + 0.587 * $color.G + 0.114 * $color.B) / 255
    $ink = if ($lum -lt 0.45) { [System.Drawing.Color]::White } else { [System.Drawing.Color]::FromArgb(255, 24, 24, 24) }
    $g.DrawString($text, $font, (New-Object System.Drawing.SolidBrush $ink), $x, $y)

    $bmp.Save((Join-Path $Out "$key.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

# The logo is the large image; it keeps the key "nanti".
Copy-Item (Join-Path $PSScriptRoot '..\assets\nanti-logo.png') (Join-Path $Out 'nanti.png') -Force
"Wrote $($badges.Count + 1) images to $((Resolve-Path $Out).Path)"
