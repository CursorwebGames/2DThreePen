import { resolve } from "path";
import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
    base: "",
    plugins: [wasm()],
    build: {
        rolldownOptions: {
            output: {
                manualChunks: id => {
                    if (id.includes('node_modules/p5')) return 'p5';
                }
            }
        }
    }
});