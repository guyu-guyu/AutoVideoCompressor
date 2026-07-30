// After `tauri build`, copy the release exe with a version suffix so
// all output artefacts (exe / msi / nsis) carry the same version string.

const fs = require("fs");
const path = require("path");

const pkg = require("../package.json");
const tauriConf = require("../src-tauri/tauri.conf.json");
const version = tauriConf.version || pkg.version;
const product = tauriConf.productName || "AutoVideoCompressor";

const releaseDir = path.join(__dirname, "..", "src-tauri", "target", "release");
const src = path.join(releaseDir, "autovideocompressor.exe");
const dst = path.join(releaseDir, `${product}_${version}.exe`);

if (!fs.existsSync(src)) {
  console.error(`rename-exe: source not found: ${src}`);
  process.exit(1);
}

fs.copyFileSync(src, dst);
console.log(`rename-exe: ${src}  →  ${dst}`);
