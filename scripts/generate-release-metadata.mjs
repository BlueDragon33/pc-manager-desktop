import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const args = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  args.set(process.argv[index], process.argv[index + 1]);
}

const directory = args.get("--dir");
const channel = args.get("--channel");
const signed = args.get("--signed") === "true";
const output = args.get("--output");

if (!directory || !output || !["stable", "beta", "dev"].includes(channel)) {
  throw new Error("Usage: --dir <path> --channel stable|beta|dev --signed true|false --output <file>");
}

const tauri = JSON.parse(fs.readFileSync("apps/desktop/src-tauri/tauri.conf.json", "utf8"));
const files = fs.readdirSync(directory).filter((name) => name.toLowerCase().endsWith(".exe")).sort();
if (files.length === 0) throw new Error("No Windows installer found for release metadata.");

const artifacts = files.map((name) => {
  const fullPath = path.join(directory, name);
  const bytes = fs.readFileSync(fullPath);
  return {
    platform: "windows-x86_64",
    kind: "nsis",
    file: name,
    sizeBytes: bytes.length,
    sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
    signed,
  };
});

const value = {
  schema: "pc-manager.update/v1",
  appId: "pc-manager",
  version: String(tauri.version),
  channel,
  publishedAt: new Date().toISOString(),
  signatureRequired: channel !== "dev",
  artifacts,
  rollback: {
    automaticDowngrade: false,
    preserveLocalState: true,
    strategy: "reinstall-previous-signed-release",
  },
};

fs.writeFileSync(output, JSON.stringify(value, null, 2) + "\n");
