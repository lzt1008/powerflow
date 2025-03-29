import { defineConfig } from 'bumpp'

export default defineConfig({
  files: [
    'package.json',
    'src-tauri/tauri.conf.json',
    'src-tauri/Cargo.toml',
    'crates/tpower/Cargo.toml',
  ],
  execute: 'cargo update powerflow tpower',
  sign: true,
  push: false,
})
