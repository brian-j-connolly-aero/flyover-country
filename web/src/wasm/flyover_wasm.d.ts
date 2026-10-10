/* tslint:disable */
/* eslint-disable */

export class Engine {
    free(): void;
    [Symbol.dispose](): void;
    advise(state: string, seat: number, orders: string): string;
    best_targets(state: string, seat: number): string;
    carry_over(state: string, seat: number): string;
    check_mission(state: string, sq: number, mission: string): string;
    /**
     * Everything the renderer needs to draw the map and write the words.
     */
    content_json(): string;
    constructor(map: string, units: string, factions: string, rules: string, patch_notes: string, flavor: string);
    new_game(config: string): string;
    /**
     * One mission preview against a draft (for drag feedback).
     */
    preview(state: string, orders: string, req: string): string;
    /**
     * Route, Fuel and risk for every flying mission in a draft.
     */
    previews(state: string, orders: string): string;
    public_view(state: string): string;
    /**
     * Resolve a night. Returns {"state": ..., "events": [...], "hash": "..."}.
     */
    resolve(state: string, orders: string): string;
    seat_view(state: string, seat: number): string;
    state_hash(state: string): string;
    suggest(state: string, seat: number, level: string): string;
    weak_spot(state: string, viewer: number, target: number): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_engine_free: (a: number, b: number) => void;
    readonly engine_advise: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly engine_best_targets: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly engine_carry_over: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly engine_check_mission: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly engine_content_json: (a: number) => [number, number, number, number];
    readonly engine_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => [number, number, number];
    readonly engine_new_game: (a: number, b: number, c: number) => [number, number, number, number];
    readonly engine_preview: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly engine_previews: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly engine_public_view: (a: number, b: number, c: number) => [number, number, number, number];
    readonly engine_resolve: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly engine_seat_view: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly engine_state_hash: (a: number, b: number, c: number) => [number, number, number, number];
    readonly engine_suggest: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly engine_weak_spot: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
