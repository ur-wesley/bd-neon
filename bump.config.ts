import fs from 'node:fs'

const cargoToml = fs.readFileSync('Cargo.toml', 'utf-8')
const version = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1]

/** @type {import('bumpp').VersionBumpOptions} */
export default {
  currentVersion: version,
  files: ['Cargo.toml'],
  execute: 'cargo check',
  all: true,
}
