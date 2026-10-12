// Saves: autosave every dawn to IndexedDB, export and import a gzipped file.
// A save is the seed, the faction picks, every order so far and a state
// snapshot per night; replaying the orders must reproduce every hash.

import type { SaveFile } from './types';

const DB = 'flyover-country';
const STORE = 'saves';

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE, { keyPath: 'id' });
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function tx<T>(mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await open();
  return new Promise((resolve, reject) => {
    const t = db.transaction(STORE, mode);
    const r = f(t.objectStore(STORE));
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}

export async function autosave(save: SaveFile): Promise<void> {
  try {
    save.updated = Date.now();
    await tx('readwrite', (s) => s.put(save));
  } catch (e) {
    console.warn('autosave failed', e);
  }
}

export async function listSaves(): Promise<SaveFile[]> {
  try {
    const all = ((await tx('readonly', (s) => s.getAll())) as SaveFile[]).filter((s) => s.version === 2);
    return all.sort((a, b) => b.updated - a.updated);
  } catch {
    return [];
  }
}

export async function deleteSave(id: string): Promise<void> {
  try {
    await tx('readwrite', (s) => s.delete(id));
  } catch {
    /* nothing to delete */
  }
}

export async function exportSave(save: SaveFile): Promise<void> {
  const json = JSON.stringify(save);
  let blob: Blob;
  let name = `flyover-night${save.nights.length}-${save.id.slice(0, 6)}.flyover`;
  if ('CompressionStream' in window) {
    const stream = new Blob([json]).stream().pipeThrough(new CompressionStream('gzip'));
    blob = await new Response(stream).blob();
  } else {
    blob = new Blob([json], { type: 'application/json' });
    name += '.json';
  }
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 5000);
}

export async function importSave(file: File): Promise<SaveFile> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let text: string;
  if (bytes[0] === 0x1f && bytes[1] === 0x8b) {
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'));
    text = await new Response(stream).text();
  } else {
    text = new TextDecoder().decode(bytes);
  }
  const save = JSON.parse(text) as SaveFile;
  if (save.version !== 2 || !Array.isArray(save.snapshots) || !save.factions) throw new Error((save as { version: number }).version === 1 ? 'That save is from the first version of the game and can’t be loaded.' : 'That file is not a Flyover Country save.');
  return save;
}
