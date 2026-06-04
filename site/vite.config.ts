import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
// @ts-ignore — rollup-plugin-obfuscator has no bundled types
import obfuscatorPlugin from 'rollup-plugin-obfuscator'

// aborn.live — static SPA.
// JS obfuscation (production only): makes the bundle unreadable so the
// wire-protocol format and interaction logic cannot be reverse-engineered
// by reading the minified source. Does NOT affect functionality.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const IS_PROD = (globalThis as any).process?.env?.NODE_ENV === 'production'

export default defineConfig({
  plugins: [
    react(),
    IS_PROD && obfuscatorPlugin({
      // Per-file obfuscation applied by rollup after Vite's own bundling.
      options: {
        compact: true,
        controlFlowFlattening: false,    // keep perf reasonable
        deadCodeInjection: false,
        debugProtection: false,
        disableConsoleOutput: true,      // remove all console.* calls
        identifierNamesGenerator: 'hexadecimal',
        renameGlobals: false,
        selfDefending: false,
        stringArray: true,
        stringArrayEncoding: ['base64'],
        stringArrayThreshold: 0.75,
        transformObjectKeys: true,
        unicodeEscapeSequence: false,
      },
    }),
  ].filter(Boolean),
  server: {
    host: true,
    port: 5173,
    proxy: {
      '/ws': {
        target: 'ws://127.0.0.1:3900',
        ws: true,
        rewriteWsOrigin: true,
      },
    },
  },
  build: {
    target: 'es2020',
    sourcemap: false,   // never ship source maps to production
  },
})
