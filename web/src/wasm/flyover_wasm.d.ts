/* tslint:disable */
/* eslint-disable */

export class Engine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Last night's missions that still make sense, as tonight's starting plan.
     */
    carry_over(state: string, prev: string): string;
    /**
     * The plan as the night will read it, what the map should draw for it,
     * and the advisor's tips.
     */
    check(state: string, orders: string): string;
    /**
     * Everything the screens need to draw the map and write the words.
     */
    content_json(): string;
    /**
     * Why a squadron can't take a mission ("" if it can).
     */
    mission_error(state: string, sq: number, mission: string): string;
    constructor(map: string, factions: string, rules: string, flavor: string);
    new_game(config: string): string;
    /**
     * Why a seat can't patrol here ("" if it can).
     */
    patrol_error(state: string, seat: number, x: number, y: number): string;
    /**
     * Resolve a night: `orders` is a JSON array with one entry per seat.
     */
    resolve(state: string, orders: string): string;
    /**
     * Why a seat can't hit this site tonight ("" if it can).
     */
    site_error(state: string, seat: number, site: number): string;
    state_hash(state: string): string;
    suggest(state: string, seat: number, level: string): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_engine_free: (a: number, b: number) => void;
    readonly engine_carry_over: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly engine_check: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly engine_content_json: (a: number) => [number, number, number, number];
    readonly engine_mission_error: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly engine_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly engine_new_game: (a: number, b: number, c: number) => [number, number, number, number];
    readonly engine_patrol_error: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly engine_resolve: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly engine_site_error: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly engine_state_hash: (a: number, b: number, c: number) => [number, number, number, number];
    readonly engine_suggest: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
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
