param([string]$Source = (Join-Path $PSScriptRoot '../assets/branding/reborn.png'))
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$sourceImage = [System.Drawing.Bitmap]::new((Resolve-Path -LiteralPath $Source).Path)
$frames = [System.Collections.Generic.List[byte[]]]::new()
$sizes = @(16,24,32,48,64,128,256)
try {
    foreach ($size in $sizes) {
        $frame = [System.Drawing.Bitmap]::new($size,$size,[System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $graphics = [System.Drawing.Graphics]::FromImage($frame)
        $stream = [System.IO.MemoryStream]::new()
        try {
            $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.DrawImage($sourceImage,0,0,$size,$size)
            $frame.Save($stream,[System.Drawing.Imaging.ImageFormat]::Png)
            $frames.Add($stream.ToArray())
            if ($size -eq 256) { [System.IO.File]::WriteAllBytes((Join-Path $PSScriptRoot '../assets/branding/reborn-ui.png'),$stream.ToArray()) }
        } finally { $stream.Dispose(); $graphics.Dispose(); $frame.Dispose() }
    }
    $iconStream = [System.IO.File]::Create((Join-Path $PSScriptRoot '../assets/branding/reborn.ico'))
    $writer = [System.IO.BinaryWriter]::new($iconStream)
    try {
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
        $offset = 6 + 16 * $sizes.Count
        for ($i=0;$i -lt $sizes.Count;$i++) {
            $sizeByte = if ($sizes[$i] -eq 256) { 0 } else { $sizes[$i] }
            $writer.Write([byte]$sizeByte); $writer.Write([byte]$sizeByte)
            $writer.Write([byte]0); $writer.Write([byte]0)
            $writer.Write([uint16]1); $writer.Write([uint16]32)
            $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
            $offset += $frames[$i].Length
        }
        foreach ($frameBytes in $frames) { $writer.Write($frameBytes) }
    } finally { $writer.Dispose() }
} finally { $sourceImage.Dispose() }
