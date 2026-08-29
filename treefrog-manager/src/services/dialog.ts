// TreeFrog Content Manager — Native desktop dialogs
// Uses @tauri-apps/plugin-dialog for Windows native pickers.
// All source folder selections must go through this service.

import { open } from "@tauri-apps/plugin-dialog";

export type PickFolderOptions = {
  title?: string;
  defaultPath?: string;
};

export type PickFileOptions = {
  title?: string;
  defaultPath?: string;
  filters?: { name: string; extensions: string[] }[];
  multiple?: boolean;
};

/**
 * Open native Windows folder picker.
 * Returns selected path or null if cancelled.
 */
export async function pickFolder(opts: PickFolderOptions = {}): Promise<string | null> {
  const result = await open({
    directory: true,
    multiple: false,
    title: opts.title ?? "Select folder",
    defaultPath: opts.defaultPath,
  });
  if (Array.isArray(result)) return result[0] ?? null;
  return (result as string | null) ?? null;
}

/**
 * Open native Windows file picker.
 * Returns selected path or null if cancelled.
 */
export async function pickFile(opts: PickFileOptions = {}): Promise<string | null> {
  const result = await open({
    directory: false,
    multiple: false,
    title: opts.title ?? "Select file",
    defaultPath: opts.defaultPath,
    filters: opts.filters,
  });
  if (Array.isArray(result)) return result[0] ?? null;
  return (result as string | null) ?? null;
}

/**
 * Open native Windows file picker with multi-select.
 */
export async function pickFiles(opts: PickFileOptions = {}): Promise<string[] | null> {
  const result = await open({
    directory: false,
    multiple: true,
    title: opts.title ?? "Select files",
    defaultPath: opts.defaultPath,
    filters: opts.filters,
  });
  if (result === null) return null;
  if (Array.isArray(result)) return result as string[];
  return [result as string];
}

/**
 * Open native Windows folder picker with multi-select.
 */
export async function pickFolders(opts: PickFolderOptions = {}): Promise<string[] | null> {
  const result = await open({
    directory: true,
    multiple: true,
    title: opts.title ?? "Select folders",
    defaultPath: opts.defaultPath,
  });
  if (result === null) return null;
  if (Array.isArray(result)) return result as string[];
  return [result as string];
}

// Re-export for convenience
export const dialogService = {
  pickFolder,
  pickFile,
  pickFiles,
  pickFolders,
};

export default dialogService;
