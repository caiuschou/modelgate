/**
 * Starts OpenAI-compatible mock + `cargo run` with cwd = e2e/ (loads e2e/config.toml).
 * Playwright `webServer` runs this and waits on /healthz.
 */
import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '..')

const mock = spawn(process.execPath, [path.join(__dirname, 'mock-openai-upstream.mjs')], {
  stdio: 'inherit',
  env: { ...process.env },
})

const rust = spawn(
  'cargo',
  ['run', '--manifest-path', path.join(repoRoot, 'Cargo.toml')],
  {
    cwd: __dirname,
    stdio: 'inherit',
    env: {
      ...process.env,
      RUST_LOG: process.env.RUST_LOG ?? 'error',
    },
  },
)

function shutdown() {
  mock.kill('SIGTERM')
  rust.kill('SIGTERM')
}

process.on('SIGINT', shutdown)
process.on('SIGTERM', shutdown)

mock.on('exit', (code, signal) => {
  if (code !== 0 && code !== null) {
    process.stderr.write(`[e2e] mock exited ${code}\n`)
  }
  rust.kill('SIGTERM')
})

rust.on('exit', (code) => {
  mock.kill('SIGTERM')
  if (code !== 0 && code !== null) {
    process.exit(code)
  }
})

rust.on('error', (err) => {
  process.stderr.write(`[e2e] failed to spawn cargo: ${err.message}\n`)
  mock.kill('SIGTERM')
  process.exit(1)
})
