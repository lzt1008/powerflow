import antfu from '@antfu/eslint-config'

export default antfu({
  ignores: [
    'src/bindings.ts',
    'src-tauri/gen/**',
    '.pnpm-store/**',
    'dist/**',
    'target/**',
    '.auto-imports/**',
  ],
  rules: {
    'vue/max-attributes-per-line': ['warn', {
      singleline: {
        max: 3,
      },
    }],
  },
}, [
  {
    name: 'ignore-tsconfig-order',
    files: ['tsconfig.json', 'tsconfig.node.json'],
    rules: {
      'jsonc/sort-keys': 'off',
    },
  },
])
