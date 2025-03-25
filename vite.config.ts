import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
	plugins: [react()],

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
					protocol: "ws",
					host,
					port: 1421,
			  }
			: undefined,
		watch: {
			// 3. tell vite to ignore watching `src-tauri`
			ignored: ["**/src-tauri/**"],
		},
	},
	envPrefix: ["VITE_", "TAURI_ENV_*"],
	build: {
		// @ts-expect-error process is a nodejs global
		target: process.env.TAURI_ENV_PLATFORM == "windows" ? "chrome105" : "safari13",
		// @ts-expect-error process is a nodejs global
		minify: !!process.env.TAURI_ENV_DEBUG,
		// produce sourcemaps for debug builds
		// @ts-expect-error process is a nodejs global
		sourcemap: !!process.env.TAURI_ENV_DEBUG,
	},
}));
