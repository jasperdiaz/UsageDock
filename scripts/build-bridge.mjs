// Builds usagedock-bridge in release mode and copies it to where Tauri's
// `externalBin` expects it (name suffixed with the Rust target triple).
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const triple = execFileSync("rustc", ["-vV"], { encoding: "utf8" })
  .split("\n")
  .find((l) => l.startsWith("host:"))
  .slice(5)
  .trim();

execFileSync("cargo", ["build", "--release", "--manifest-path", join(root, "bridge", "Cargo.toml")], {
  stdio: "inherit",
});

const ext = process.platform === "win32" ? ".exe" : "";
const outDir = join(root, "src-tauri", "binaries");
mkdirSync(outDir, { recursive: true });
copyFileSync(
  join(root, "bridge", "target", "release", `usagedock-bridge${ext}`),
  join(outDir, `usagedock-bridge-${triple}${ext}`),
);
console.log(`bridge -> src-tauri/binaries/usagedock-bridge-${triple}${ext}`);
