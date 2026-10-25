import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, loadEnv } from "vite";
import packageJson from "./package.json" with { type: "json" };

const __dirname = dirname(fileURLToPath(import.meta.url));

function swVersionPlugin() {
  return {
    name: "sw-version-plugin",
    closeBundle() {
      const distSw = resolve(__dirname, "dist/sw.js");
      if (existsSync(distSw)) {
        let content = readFileSync(distSw, "utf-8");
        content = content.replace(
          /const CACHE_VERSION = "[^"]*";/,
          `const CACHE_VERSION = "${packageJson.version}";`
        );
        writeFileSync(distSw, content, "utf-8");
      }
    },
  };
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const apiTarget = env.VITE_API_PROXY_TARGET || "http://127.0.0.1:12000";

  return {
    plugins: [react(), tailwindcss(), swVersionPlugin()],
    server: {
      host: "0.0.0.0",
      port: 5173,
      proxy: {
        "/api": { target: apiTarget, changeOrigin: true },
        "/health": { target: apiTarget, changeOrigin: true },
      },
    },
    preview: { host: "0.0.0.0", port: 4173 },
  };
});
