import { defineConfig } from 'vite';

// One Netscript script in, one file out. Nothing is transpiled: Bitburner
// 3.x ships Electron 41 (Chromium 146), which runs everything tsc emits.
export default defineConfig({
    build: {
        target: 'esnext',
        lib: {
            entry: 'src/main.ts',
            formats: ['es'],
            fileName: () => 'mcp.js',
        },
        // Readable in the in-game editor. RAM is charged per identifier
        // referenced, not per byte, so minifying buys nothing.
        minify: false,
        sourcemap: false,
        rolldownOptions: {
            // The game loads one blob-URL module; a second chunk would
            // never be found. Keep every dynamic import inlined.
            output: { codeSplitting: false },
        },
    },
});
