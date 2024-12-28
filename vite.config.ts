import { execSync } from 'node:child_process'
import process from 'node:process'
import { fileURLToPath, URL } from 'node:url'

import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import autoprefixer from 'autoprefixer'
import { format } from 'date-fns'
import tailwind from 'tailwindcss'
import autoImport from 'unplugin-auto-import/vite'
import components from 'unplugin-vue-components/vite'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST

const commitHash = execSync('git rev-parse HEAD').toString().trim()

// https://vitejs.dev/config/
export default defineConfig(async mode => ({
  plugins: [
    vue(),
    vueJsx(),
    autoImport({
      imports: ['vue', 'pinia', '@vueuse/core'],
      dirsScanOptions: {
        types: true,
      },
      defaultExportByFilename: true,
      dirs: [
        './src/composables/**',
        './src/components/**',
        './src/stores/**',
      ],
      dts: './.auto-imports/auto-imports.d.ts',
      vueTemplate: true,
    }),
    components({
      dts: './.auto-imports/components.d.ts',
    }),
  ],
  css: {
    postcss: {
      plugins: [tailwind(), autoprefixer()],
    },
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  define: {
    __BUILD_DATE__: JSON.stringify(format(new Date(), 'yyyy-MM-dd')),
    __COMMIT_HASH__: JSON.stringify(commitHash),
  },
  build: {
    rollupOptions: {
      input: mode.command === 'build'
        ? {
            main: './index.html',
            popover: './popover.html',
            settings: './settings.html',
          }
        : undefined,
    },
  },
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}))
