import { resolve } from "path";
import { defineConfig } from "vite";

export default defineConfig({
    base: "",
    // build: {
    //     rolldownOptions: {
    //         output: {
    //             manualChunks: id => {
    //                 if (id.includes('node_modules/p5')) return 'p5';
    //             }
    //         }
    //     }
    // }
});