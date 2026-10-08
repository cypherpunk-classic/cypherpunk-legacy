import { defineConfig } from 'vite';
import { viteSingleFile } from 'vite-plugin-singlefile';

// Two build targets:
//   npm run build         -> dist/        multi-file static site (GitHub Pages, any static host)
//   npm run build:single  -> dist-single/ one self-contained index.html (works from file://)
export default defineConfig(({ mode }) => {
  const single = mode === 'single';
  return {
    base: './',
    plugins: single ? [viteSingleFile({ removeViteModuleLoader: true })] : [],
    build: {
      outDir: single ? 'dist-single' : 'dist',
      target: 'es2022',
      assetsInlineLimit: single ? 100_000_000 : 4096,
      chunkSizeWarningLimit: 2500,
      sourcemap: !single,
    },
    test: {
      include: ['tests/unit/**/*.test.ts'],
      environment: 'node',
      testTimeout: 60_000,
    },
  };
});
