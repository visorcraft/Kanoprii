import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const root = path.dirname(fileURLToPath(import.meta.url))
const wdioTauriPlugin = path.join(root, 'e2e/node_modules/@wdio/tauri-plugin')

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: fs.existsSync(wdioTauriPlugin)
      ? { '@wdio/tauri-plugin': wdioTauriPlugin }
      : {},
  },
})