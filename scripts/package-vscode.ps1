# Packages the VS Code extension (editors/vscode) as a .vsix, without npm:
# the extension uses no npm packages, so the .vsix is its files plus two
# small manifests. Writes .kavach/kavach-vscode-<version>.vsix.
#
#   powershell -File scripts/package-vscode.ps1
#   code --install-extension .kavach/kavach-vscode-<version>.vsix

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$ext = Join-Path $root 'editors/vscode'
$package = Get-Content (Join-Path $ext 'package.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$version = $package.version
$description = [System.Security.SecurityElement]::Escape($package.description)

$outDir = Join-Path $root '.kavach'
New-Item -ItemType Directory -Force $outDir | Out-Null
$out = Join-Path $outDir "kavach-vscode-$version.vsix"
if (Test-Path $out) { Remove-Item $out -Force }

# The extension's files: what VS Code needs, not its tests.
$files = @('package.json', 'extension.js', 'language-configuration.json', 'README.md', 'client/rpc.js')
foreach ($dir in @('syntaxes', 'icons')) {
    Get-ChildItem (Join-Path $ext $dir) -File | ForEach-Object { $files += "$dir/$($_.Name)" }
}

$types = @'
<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension=".json" ContentType="application/json"/>
  <Default Extension=".js" ContentType="application/javascript"/>
  <Default Extension=".md" ContentType="text/markdown"/>
  <Default Extension=".svg" ContentType="image/svg+xml"/>
  <Default Extension=".vsixmanifest" ContentType="text/xml"/>
</Types>
'@
$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Language="en-US" Id="kavach" Version="$version" Publisher="kavach"/>
    <DisplayName>Kavach</DisplayName>
    <Description xml:space="preserve">$description</Description>
    <Categories>Programming Languages</Categories>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="^1.75.0"/>
    </Properties>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
  </Assets>
</PackageManifest>
"@

Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open($out, 'Create')
try {
    $utf8 = New-Object System.Text.UTF8Encoding $false
    foreach ($entry in @(@('[Content_Types].xml', $types), @('extension.vsixmanifest', $manifest))) {
        $writer = New-Object System.IO.StreamWriter($zip.CreateEntry($entry[0]).Open(), $utf8)
        $writer.Write($entry[1])
        $writer.Dispose()
    }
    # Paths inside a .vsix use `/`, whatever the system.
    foreach ($f in $files) {
        $source = Join-Path $ext $f
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $source, "extension/$f") | Out-Null
    }
} finally {
    $zip.Dispose()
}
Write-Output "built $out"
Write-Output "install it with: code --install-extension `"$out`""
