import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';
// import { internalIpV4 } from 'internal-ip';

const host = process.env.TAURI_DEV_HOST;

const ReactCompilerConfig = {};

// https://vitejs.dev/config/
export default defineConfig(async ({ command, mode }) => {
  console.log({ command, mode, nodeEnv: process.env.NODE_ENV });

  const isAndroid = mode === 'ad';
  const isDev = mode === 'development';
  // const host = isAndroid ? await internalIpV4() : 'localhost';

  /** @type {import('vite').UserConfig} */
  const config: import('vite').UserConfig = {
    base: './',
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    css: {
      modules: {
        localsConvention: 'camelCaseOnly',
      },
    },
    plugins: [
      react({
        babel: {
          plugins: ['babel-plugin-react-compiler', ReactCompilerConfig],
        },
      }),
      tailwindcss(),
    ],

    build: {
      sourcemap: isDev,
      target: ['es2015'],
      // minify: 'terser',
      cssMinify: 'lightningcss',
      rollupOptions: {
        output: {
          advancedChunks: {
            groups: [
              {
                name: 'theme',
                test: 'src/styles/theme.ts',
              },
              {
                name: 'logger',
                test: 'src/utils/logger.ts',
              },
              {
                name: 'eruda',
                test: 'src/utils/eruda.js',
              },
              { name: 'mock', test: 'mocks/browser.js' },
              {
                name: 'lodash',
                test: /lodash/,
              },
              {
                name: 'tailwind',
                test: 'src/styles/tailwind.css',
              },
              {
                name: 'icons',
                test: 'react-icons',
              },
              {
                name: 'scrollbar',
                test: 'simplebar-react',
              },
              {
                name: 'router',
                test: 'react-router',
              },
              {
                // 包含react和react-dom
                name: 'react',
                test: /(react|react-dom)/,
              },
            ],
          },
        },
      },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 2025,
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
    experimental: {
      enableNativePlugin: false,
    },
  };

  return config;
});
