import { spawnSync } from "node:child_process";

const runner = "npx";

function run(args) {
  const result = spawnSync(runner, args, {
    stdio: "inherit",
    shell: process.platform === "win32",
  });

  if (typeof result.status === "number" && result.status !== 0) {
    process.exit(result.status);
  }

  if (result.error) {
    console.error(result.error);
    process.exit(1);
  }
}

run(["tsc", "--pretty", "false"]);
run(["vite", "build"]);
