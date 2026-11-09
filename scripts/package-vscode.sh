#!/bin/sh
# Packages the VS Code extension (editors/vscode) as a .vsix, without npm:
# the extension uses no npm packages, so the .vsix is its files plus two
# small manifests. Writes .kavach/kavach-vscode-<version>.vsix. Needs
# `zip`, or Python 3 when there is no `zip`.
#
#   sh scripts/package-vscode.sh
#   code --install-extension .kavach/kavach-vscode-<version>.vsix

set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
ext="$root/editors/vscode"
version=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ext/package.json")
description=$(sed -n 's/^  "description": "\(.*\)",$/\1/p' "$ext/package.json" |
    sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')
out="$root/.kavach/kavach-vscode-$version.vsix"
stage="$root/.kavach/vscode-package"

rm -rf "$stage" "$out"
mkdir -p "$stage/extension/client"
# The extension's files: what VS Code needs, not its tests.
cp "$ext/package.json" "$ext/extension.js" "$ext/language-configuration.json" \
    "$ext/README.md" "$stage/extension/"
cp "$ext/client/rpc.js" "$stage/extension/client/"
cp -r "$ext/syntaxes" "$ext/icons" "$stage/extension/"

cat > "$stage/[Content_Types].xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension=".json" ContentType="application/json"/>
  <Default Extension=".js" ContentType="application/javascript"/>
  <Default Extension=".md" ContentType="text/markdown"/>
  <Default Extension=".svg" ContentType="image/svg+xml"/>
  <Default Extension=".vsixmanifest" ContentType="text/xml"/>
</Types>
EOF
cat > "$stage/extension.vsixmanifest" <<EOF
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
EOF

if command -v zip >/dev/null 2>&1; then
    (cd "$stage" && zip -qr "$out" '[Content_Types].xml' extension.vsixmanifest extension)
else
    python=$(command -v python3 || command -v python)
    "$python" -I -c 'import shutil, sys; shutil.make_archive(sys.argv[1], "zip", sys.argv[2])' \
        "$out.tmp" "$stage"
    mv "$out.tmp.zip" "$out"
fi
rm -rf "$stage"
echo "built $out"
echo "install it with: code --install-extension \"$out\""
