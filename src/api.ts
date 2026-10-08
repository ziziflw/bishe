import { invoke } from '@tauri-apps/api/core';
export interface Tag { id: number; name: string }
export type Status = 'READY' | 'MISSING' | 'UNAUTHORIZED' | 'CHANGED' | 'ERROR';
export interface FileRecord {
  id: number; identity: string; path: string; name: string; extension: string;
  size: number; modified_at: number; registered_at: number; status: Status; tags: Tag[];
}
export interface Grant { id: number; kind: 'FILE' | 'DIRECTORY'; path: string; identity: string; active: boolean }
export interface ImportReport { imported: number; errors: string[] }
export const api = {
  search: (query = '', tagId: number | null = null, extension = '') => invoke<FileRecord[]>('search_files', { query, tagId, extension }),
  tags: () => invoke<Tag[]>('list_tags'),
  grants: () => invoke<Grant[]>('list_grants'),
  importFiles: () => invoke<ImportReport>('import_files'),
  authorizeDirectory: () => invoke<boolean>('authorize_directory'),
  revoke: (id: number) => invoke<void>('revoke_grant', { id }),
  details: (id: number) => invoke<FileRecord>('file_details', { id }),
  createTag: (name: string) => invoke<void>('create_tag', { name }),
  renameTag: (id: number, name: string) => invoke<void>('rename_tag', { id, name }),
  setTag: (fileId: number, tagId: number, attached: boolean) => invoke<void>('set_file_tag', { fileId, tagId, attached }),
  relink: (id: number) => invoke<boolean>('relink_file', { id }),
  deleteIndex: (id: number) => invoke<void>('delete_index', { id }),
};
export function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') return error.message;
  return '操作未完成。请确认在桌面应用中运行，并稍后重试。';
}
