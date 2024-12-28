import antfu from '@antfu/eslint-config'

export default antfu({
  ignores: ['src/bindings.ts'],
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
