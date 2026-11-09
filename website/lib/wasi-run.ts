// Runs a WASI (preview 1) program in the browser: standard input from a
// byte array, standard output and error collected into text. Just enough
// of WASI for the playground engine (decision 0078) — a program that
// reads input, computes and prints; anything else it asks for answers
// "not supported".

export type WasiResult = { stdout: string; stderr: string; exit: number };

const BADF = 8; // WASI errno: bad file descriptor.
const NOSYS = 52; // WASI errno: not supported.

export async function runWasi(
  module: WebAssembly.Module,
  stdin: Uint8Array,
): Promise<WasiResult> {
  const box: { memory?: WebAssembly.Memory } = {};
  const out: number[] = [];
  const err: number[] = [];
  let stdinAt = 0;
  let exit = 0;
  const stop = { exited: true };

  const view = () => new DataView(box.memory!.buffer);
  const bytes = () => new Uint8Array(box.memory!.buffer);

  // An iovec list: pairs of (pointer, length) in the program's memory.
  const iovList = (iovs: number, count: number): Array<[number, number]> => {
    const v = view();
    const list: Array<[number, number]> = [];
    for (let i = 0; i < count; i++) {
      list.push([v.getUint32(iovs + i * 8, true), v.getUint32(iovs + i * 8 + 4, true)]);
    }
    return list;
  };

  const impls: Record<string, (...args: number[]) => number> = {
    fd_read(fd, iovs, count, nreadPtr) {
      if (fd !== 0) return BADF;
      let n = 0;
      for (const [ptr, len] of iovList(iovs, count)) {
        const take = Math.min(len, stdin.length - stdinAt);
        bytes().set(stdin.subarray(stdinAt, stdinAt + take), ptr);
        stdinAt += take;
        n += take;
        if (take < len) break;
      }
      view().setUint32(nreadPtr, n, true);
      return 0;
    },
    fd_write(fd, iovs, count, nwrittenPtr) {
      if (fd !== 1 && fd !== 2) return BADF;
      const sink = fd === 1 ? out : err;
      let n = 0;
      for (const [ptr, len] of iovList(iovs, count)) {
        const b = bytes();
        for (let i = 0; i < len; i++) sink.push(b[ptr + i]);
        n += len;
      }
      view().setUint32(nwrittenPtr, n, true);
      return 0;
    },
    environ_sizes_get(countPtr, sizePtr) {
      view().setUint32(countPtr, 0, true);
      view().setUint32(sizePtr, 0, true);
      return 0;
    },
    environ_get: () => 0,
    args_sizes_get(countPtr, sizePtr) {
      view().setUint32(countPtr, 0, true);
      view().setUint32(sizePtr, 0, true);
      return 0;
    },
    args_get: () => 0,
    random_get(ptr, len) {
      crypto.getRandomValues(bytes().subarray(ptr, ptr + len));
      return 0;
    },
    clock_time_get(_id, _precision, timePtr) {
      view().setBigUint64(timePtr, BigInt(Date.now()) * BigInt(1e6), true);
      return 0;
    },
    // Zeroed: file type unknown, no flags and no rights, which standard
    // output and input accept.
    fd_fdstat_get(_fd, statPtr) {
      bytes().fill(0, statPtr, statPtr + 24);
      return 0;
    },
    fd_prestat_get: () => BADF,
    fd_close: () => 0,
    proc_exit(code) {
      exit = code;
      throw stop;
    },
  };

  // The import object is built from what the module asks for, so a new
  // compiler version that imports one more call still instantiates.
  const imports: Record<string, Record<string, unknown>> = {};
  for (const im of WebAssembly.Module.imports(module)) {
    if (im.kind !== "function") continue;
    (imports[im.module] ??= {})[im.name] = impls[im.name] ?? (() => NOSYS);
  }
  const instance = await WebAssembly.instantiate(module, imports as WebAssembly.Imports);
  box.memory = instance.exports.memory as WebAssembly.Memory;
  try {
    (instance.exports._start as () => void)();
  } catch (e) {
    if (e !== stop) throw e;
  }
  const text = new TextDecoder();
  return {
    stdout: text.decode(Uint8Array.from(out)),
    stderr: text.decode(Uint8Array.from(err)),
    exit,
  };
}
