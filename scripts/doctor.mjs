// Checks the Windows build prerequisites before anything heavy runs.
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";

const bad = [];
const ok = (msg) => console.log(`  ok   ${msg}`);
const fail = (msg, fix) => {
  console.log(`  FAIL ${msg}`);
  bad.push(fix);
};

function has(cmd) {
  try {
    execFileSync(process.platform === "win32" ? "where" : "which", [cmd], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

function sdkLib(arch) {
  const root = join(process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)", "Windows Kits", "10", "Lib");
  if (!existsSync(root)) return null;
  for (const ver of readdirSync(root).sort().reverse()) {
    const lib = join(root, ver, "um", arch, "kernel32.lib");
    if (existsSync(lib)) return ver;
  }
  return null;
}

function msvc() {
  const vswhere = join(
    process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)",
    "Microsoft Visual Studio",
    "Installer",
    "vswhere.exe",
  );
  if (!existsSync(vswhere)) return null;
  try {
    const out = execFileSync(vswhere, [
      "-latest",
      "-products",
      "*",
      "-requires",
      "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
      "-property",
      "installationPath",
    ]).toString().trim();
    return out || null;
  } catch {
    return null;
  }
}

console.log("KestrelUI build doctor");

if (process.platform !== "win32") {
  console.log("  skip native checks: KestrelUI builds only on Windows");
  process.exit(0);
}

const arch = process.arch === "arm64" ? "arm64" : "x64";
const vs = msvc();
if (vs) ok(`MSVC build tools: ${vs}`);
else {
  fail(
    "MSVC build tools not found",
    "Install Visual Studio Build Tools 2022 with the \"Desktop development with C++\" workload " +
      "(winget install Microsoft.VisualStudio.2022.BuildTools --override \"--wait --passive " +
      "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\")",
  );
}

const sdk = sdkLib(arch);
if (sdk) ok(`Windows SDK ${sdk} (${arch})`);
else {
  fail(
    `Windows SDK libraries (kernel32.lib for ${arch}) not found`,
    "Add a Windows 10/11 SDK in Visual Studio Installer → Build Tools → Modify → Individual components",
  );
}

for (const [cmd, fix] of [
  ["rustup", "Install Rust: https://rustup.rs (the pinned nightly installs itself on first build)"],
  ["deno", "Install Deno 2: winget install DenoLand.Deno"],
]) {
  if (has(cmd)) ok(cmd);
  else fail(`${cmd} not found in PATH`, fix);
}

if (bad.length) {
  console.log("\nFix these, open a new terminal and run `npm install` again:");
  for (const fix of bad) console.log(`  - ${fix}`);
  process.exit(1);
}
