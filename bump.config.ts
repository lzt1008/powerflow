import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { defineConfig } from 'bumpp'

export default defineConfig({
  files: [
    'package.json',
    'src-tauri/tauri.conf.json',
  ],
  execute({ state }) {
    for (const file of ['src-tauri/Cargo.toml', 'crates/tpower/Cargo.toml']) {
      const content = readFileSync(file, 'utf8')
      writeFileSync(file, content.replace(/^version = ".*"$/m, `version = "${state.newVersion}"`))
      state.updatedFiles.push(file)
    }
    execFileSync('cargo', ['update', 'powerflow', 'tpower'], { stdio: 'inherit' })
    state.updatedFiles.push('Cargo.lock')
  },
  sign: true,
  push: true,
  pr: {
    base: 'main',
    title: 'chore: release {tag}',
  },
})
