import { addPluginListener, invoke, type PluginListener } from '@tauri-apps/api/core'

const PLUGIN_NAME = 'icloud-container'
const COMMAND_PREFIX = `plugin:${PLUGIN_NAME}|`
const textEncoder = new TextEncoder()

export const DIRECTORY_CHANGED_EVENT = 'icloud://directory-changed'
export const FILE_CHANGED_EVENT = 'icloud://file-changed'

export type FileEncoding = 'utf8' | 'bytes'
export type FileProtectionType =
  | 'complete'
  | 'completeUnlessOpen'
  | 'completeUntilFirstUserAuth'
  | 'none'

export interface ContainerStatus {
  available: boolean
  reason?: string | null
}

export interface Utf8FileContent {
  encoding: 'utf8'
  content: string
}

export interface BytesFileContent {
  encoding: 'bytes'
  content: Uint8Array
}

export type FileContent = Utf8FileContent | BytesFileContent

export interface FolderEntry {
  name: string
  path: string
  isDirectory: boolean
  size?: number | null
  modifiedDate?: number | null
  createdDate?: number | null
  syncStatus?: SyncStatus | null
}

export interface ItemExistence {
  exists: boolean
  isDirectory: boolean
}

export interface SyncStatus {
  phase: 'current' | 'notDownloaded' | 'downloaded'
  isDownloading: boolean
  isUploading: boolean
  isUploaded: boolean
  downloadError?: string | null
  uploadError?: string | null
}

export interface ItemAttributes {
  size: number
  modifiedDate: number
  createdDate: number
  type: 'file' | 'dir'
  syncStatus?: SyncStatus | null
}

export interface TrashItemResult {
  path: string
}

export interface WriteFileOptions {
  encoding?: FileEncoding
  overwrite?: boolean
  fileProtection?: FileProtectionType
}

export interface ReadFileOptions {
  encoding?: FileEncoding
}

export interface CreateFileOptions {
  content?: string | Uint8Array
  encoding?: FileEncoding
  fileProtection?: FileProtectionType
}

export interface CreateDirectoryOptions {
  withIntermediateDirectories?: boolean
  fileProtection?: FileProtectionType
}

export interface ListDirectoryOptions {
  recursive?: boolean
  skipsHiddenFiles?: boolean
}

export interface DirectoryWatchEvent {
  watchId: string
  path: string
  recursive: boolean
  entries: string[]
}

export interface FileWatchEvent {
  watchId: string
  path: string
}

export type ContainerIdentifier = string | undefined

export interface ICloudContainerHandle {
  readonly identifier: string
  getStatus(): Promise<ContainerStatus>
  getUrl(): Promise<string>
  readFile(path: string, options?: ReadFileOptions): Promise<FileContent>
  writeFile(path: string, content: string | Uint8Array, options?: WriteFileOptions): Promise<void>
  createFile(path: string, options?: CreateFileOptions): Promise<FolderEntry>
  itemExists(path: string): Promise<ItemExistence>
  getAttributes(path: string): Promise<ItemAttributes>
  createDirectory(path: string, options?: CreateDirectoryOptions): Promise<void>
  listDirectory(path: string, options?: ListDirectoryOptions): Promise<FolderEntry[]>
  deleteItem(path: string): Promise<void>
  trashItem(path: string): Promise<TrashItemResult>
  moveItem(sourcePath: string, destinationPath: string): Promise<FolderEntry>
  copyItem(sourcePath: string, destinationPath: string): Promise<FolderEntry>
  getItemSyncStatus(path: string): Promise<SyncStatus>
  startDownload(path: string): Promise<void>
  evictItem(path: string): Promise<void>
  isUbiquitous(path: string): Promise<boolean>
  watchDirectory(path: string, recursive?: boolean): Promise<string>
  watchFile(path: string): Promise<string>
}

function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke(`${COMMAND_PREFIX}${command}`, args)
}

function inferEncoding(content: string | Uint8Array | undefined, encoding?: FileEncoding): FileEncoding {
  if (encoding) {
    return encoding
  }

  return content instanceof Uint8Array ? 'bytes' : 'utf8'
}

function toCreateFilePayload(options?: CreateFileOptions): {
  content?: Uint8Array
  encoding?: FileEncoding
  fileProtection?: FileProtectionType
} | undefined {
  if (!options) {
    return undefined
  }

  const encoding = inferEncoding(options.content, options.encoding)
  const content =
    typeof options.content === 'string'
      ? textEncoder.encode(options.content)
      : options.content

  return {
    content,
    encoding,
    fileProtection: options.fileProtection,
  }
}

function toWriteFilePayload(content: string | Uint8Array, encoding?: FileEncoding): FileContent {
  const resolvedEncoding = inferEncoding(content, encoding)

  if (content instanceof Uint8Array) {
    if (resolvedEncoding !== 'bytes') {
      throw new Error('Uint8Array content requires encoding "bytes"')
    }

    return { encoding: 'bytes', content }
  }

  if (resolvedEncoding === 'bytes') {
    return { encoding: 'bytes', content: textEncoder.encode(content) }
  }

  return { encoding: 'utf8', content }
}

export async function getContainerStatus(identifier?: string): Promise<ContainerStatus> {
  return invokeCommand('get_container_status', { identifier })
}

export async function getContainerUrl(identifier?: string): Promise<string> {
  return invokeCommand('get_container_url', { identifier })
}

export async function readFile(
  path: string,
  options?: ReadFileOptions,
  identifier?: ContainerIdentifier,
): Promise<FileContent> {
  return invokeCommand('read_file', { path, identifier, options })
}

export async function writeFile(
  path: string,
  content: string | Uint8Array,
  options?: WriteFileOptions,
  identifier?: ContainerIdentifier,
): Promise<void> {
  return invokeCommand('write_file', {
    path,
    identifier,
    content: toWriteFilePayload(content, options?.encoding),
    options,
  })
}

export async function createFile(
  path: string,
  options?: CreateFileOptions,
  identifier?: ContainerIdentifier,
): Promise<FolderEntry> {
  return invokeCommand('create_file', {
    path,
    identifier,
    options: toCreateFilePayload(options),
  })
}

export async function itemExists(
  path: string,
  identifier?: ContainerIdentifier,
): Promise<ItemExistence> {
  return invokeCommand('item_exists', { path, identifier })
}

export async function getAttributes(
  path: string,
  identifier?: ContainerIdentifier,
): Promise<ItemAttributes> {
  return invokeCommand('get_attributes', { path, identifier })
}

export async function createDirectory(
  path: string,
  options?: CreateDirectoryOptions,
  identifier?: ContainerIdentifier,
): Promise<void> {
  return invokeCommand('create_directory', { path, identifier, options })
}

export async function listDirectory(
  path: string,
  options?: ListDirectoryOptions,
  identifier?: ContainerIdentifier,
): Promise<FolderEntry[]> {
  return invokeCommand('list_directory', { path, identifier, options })
}

export async function deleteItem(path: string, identifier?: ContainerIdentifier): Promise<void> {
  return invokeCommand('delete_item', { path, identifier })
}

export async function trashItem(
  path: string,
  identifier?: ContainerIdentifier,
): Promise<TrashItemResult> {
  return invokeCommand('trash_item', { path, identifier })
}

export async function moveItem(
  sourcePath: string,
  destinationPath: string,
  identifier?: ContainerIdentifier,
): Promise<FolderEntry> {
  return invokeCommand('move_item', { sourcePath, destinationPath, identifier })
}

export async function copyItem(
  sourcePath: string,
  destinationPath: string,
  identifier?: ContainerIdentifier,
): Promise<FolderEntry> {
  return invokeCommand('copy_item', { sourcePath, destinationPath, identifier })
}

export async function getItemSyncStatus(
  path: string,
  identifier?: ContainerIdentifier,
): Promise<SyncStatus> {
  return invokeCommand('get_item_sync_status', { path, identifier })
}

export async function startDownload(path: string, identifier?: ContainerIdentifier): Promise<void> {
  return invokeCommand('start_download', { path, identifier })
}

export async function evictItem(path: string, identifier?: ContainerIdentifier): Promise<void> {
  return invokeCommand('evict_item', { path, identifier })
}

export async function isUbiquitous(
  path: string,
  identifier?: ContainerIdentifier,
): Promise<boolean> {
  return invokeCommand('is_ubiquitous', { path, identifier })
}

export async function watchDirectory(
  path: string,
  recursive = false,
  identifier?: ContainerIdentifier,
): Promise<string> {
  return invokeCommand('watch_directory', { path, recursive, identifier })
}

export async function unwatch(watchId: string): Promise<void> {
  return invokeCommand('unwatch', { watchId })
}

export async function watchFile(path: string, identifier?: ContainerIdentifier): Promise<string> {
  return invokeCommand('watch_file', { path, identifier })
}

export async function unwatchFile(watchId: string): Promise<void> {
  return invokeCommand('unwatch_file', { watchId })
}

export async function onDirectoryChanged(
  handler: (event: DirectoryWatchEvent) => void,
): Promise<PluginListener> {
  return addPluginListener<DirectoryWatchEvent>(PLUGIN_NAME, DIRECTORY_CHANGED_EVENT, handler)
}

export async function onFileChanged(
  handler: (event: FileWatchEvent) => void,
): Promise<PluginListener> {
  return addPluginListener<FileWatchEvent>(PLUGIN_NAME, FILE_CHANGED_EVENT, handler)
}

export function forContainer(identifier: string): ICloudContainerHandle {
  const trimmedIdentifier = identifier.trim()
  if (!trimmedIdentifier) {
    throw new Error('identifier is required and cannot be empty')
  }

  return {
    identifier: trimmedIdentifier,
    getStatus: () => getContainerStatus(trimmedIdentifier),
    getUrl: () => getContainerUrl(trimmedIdentifier),
    readFile: (path, options) => readFile(path, options, trimmedIdentifier),
    writeFile: (path, content, options) => writeFile(path, content, options, trimmedIdentifier),
    createFile: (path, options) => createFile(path, options, trimmedIdentifier),
    itemExists: (path) => itemExists(path, trimmedIdentifier),
    getAttributes: (path) => getAttributes(path, trimmedIdentifier),
    createDirectory: (path, options) => createDirectory(path, options, trimmedIdentifier),
    listDirectory: (path, options) => listDirectory(path, options, trimmedIdentifier),
    deleteItem: (path) => deleteItem(path, trimmedIdentifier),
    trashItem: (path) => trashItem(path, trimmedIdentifier),
    moveItem: (sourcePath, destinationPath) => moveItem(sourcePath, destinationPath, trimmedIdentifier),
    copyItem: (sourcePath, destinationPath) => copyItem(sourcePath, destinationPath, trimmedIdentifier),
    getItemSyncStatus: (path) => getItemSyncStatus(path, trimmedIdentifier),
    startDownload: (path) => startDownload(path, trimmedIdentifier),
    evictItem: (path) => evictItem(path, trimmedIdentifier),
    isUbiquitous: (path) => isUbiquitous(path, trimmedIdentifier),
    watchDirectory: (path, recursive) => watchDirectory(path, recursive, trimmedIdentifier),
    watchFile: (path) => watchFile(path, trimmedIdentifier),
  }
}
