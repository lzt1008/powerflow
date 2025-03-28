import { defineConfig } from 'bumpp'

export default defineConfig({
  files: [
    'package.json',
    'src-tauri/tauri.conf.json',
    'src-tauri/Cargo.toml',
    'crates/tpower/Cargo.toml',
  ],
  sign: true,
  push: false,
})
