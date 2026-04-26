import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const guestApi = fs.readFileSync(path.join(root, 'guest-js/index.ts'), 'utf8')
const defaultAcl = fs.readFileSync(path.join(root, 'permissions/default.toml'), 'utf8')
const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'))

for (const symbol of [
  'export async function getContainerStatus',
  'export async function getContainerUrl',
  'export async function readFile',
  'export async function writeFile',
  'export async function createFile',
  'export async function itemExists',
  'export async function getAttributes',
  'export async function createDirectory',
  'export async function listDirectory',
  'export async function deleteItem',
  'export async function trashItem',
  'export async function moveItem',
  'export async function copyItem',
  'export async function getItemSyncStatus',
  'export async function startDownload',
  'export async function evictItem',
  'export async function isUbiquitous',
  'export async function watchDirectory',
  'export async function unwatch',
  'export async function watchFile',
  'export async function unwatchFile',
  'export async function onDirectoryChanged',
  'export async function onFileChanged',
  'export function forContainer',
]) {
  assert.ok(guestApi.includes(symbol), `missing guest API symbol: ${symbol}`)
}

for (const typeName of [
  'export type ContainerIdentifier',
  'export interface ICloudContainerHandle',
  'export type FileEncoding',
  'export type FileProtectionType',
  'export interface FolderEntry',
  'export interface ItemAttributes',
  'export interface SyncStatus',
  'export interface DirectoryWatchEvent',
  'export interface FileWatchEvent',
]) {
  assert.ok(guestApi.includes(typeName), `missing guest API type: ${typeName}`)
}

assert.ok(guestApi.includes('addPluginListener'), 'expected plugin listener helper usage')
assert.ok(!guestApi.toLowerCase().includes('base64'), 'guest API must not introduce base64 handling')
assert.ok(guestApi.includes('identifier?: ContainerIdentifier'), 'expected guest API identifier overrides')
assert.ok(guestApi.includes("throw new Error('identifier is required and cannot be empty')"), 'expected identifier validation in container wrapper')
assert.ok(guestApi.includes('readFile: (path, options) => readFile(path, options, trimmedIdentifier)'), 'expected bound readFile wrapper')
assert.ok(guestApi.includes('watchDirectory: (path, recursive) => watchDirectory(path, recursive, trimmedIdentifier)'), 'expected bound watchDirectory wrapper')

for (const permission of [
  'allow-get-container-status',
  'allow-get-container-url',
  'allow-read-file',
  'allow-write-file',
  'allow-create-file',
  'allow-item-exists',
  'allow-get-attributes',
  'allow-create-directory',
  'allow-list-directory',
  'allow-delete-item',
  'allow-trash-item',
  'allow-move-item',
  'allow-copy-item',
  'allow-get-item-sync-status',
  'allow-start-download',
  'allow-evict-item',
  'allow-is-ubiquitous',
  'allow-watch-directory',
  'allow-unwatch',
  'allow-watch-file',
  'allow-unwatch-file',
]) {
  assert.ok(defaultAcl.includes(permission), `default ACL missing ${permission}`)
}

assert.equal(packageJson.exports['.'], './guest-js/index.ts')
assert.equal(packageJson.scripts['test:guest-api'], 'node ./tests/guest_api_contract.mjs')
assert.equal(packageJson.private, undefined)
assert.deepEqual(packageJson.files, ['guest-js', 'README.md'])