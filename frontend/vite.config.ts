import react from "@vitejs/plugin-react";
import { defineConfig } from "vite-plus";

export default defineConfig({
  plugins: [react()],
  server: {
    host: "0.0.0.0",
    port: 5173,
    proxy: {
      "/api": "http://localhost:3000",
      "/health": "http://localhost:3000",
    },
  },
  test: {
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    environment: "node",
  },
  lint: { options: { typeAware: true, typeCheck: true } },
});
