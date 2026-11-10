/*
 * The starter of a packed program (spec 10.5, decision 0081). A packed
 * file is this starter, then the compressed program, then a 40-byte
 * trailer (`src/lib.rs` describes it). The starter unpacks the program
 * once into the cache folder, named by its hash and length, and runs
 * that copy with the same arguments and environment: Linux and macOS
 * replace this process with it (execv); Windows runs it, waits, and ends
 * with its exit code, ignoring Ctrl+C itself so the program alone
 * decides how to stop.
 *
 * Written in C so it stays small: a Rust program carries 100-300 KB of
 * its standard library, more than a small packed program saves.
 */

#ifndef _WIN32
/* POSIX's `readlink`, `fseeko` and `ftello`, with 64-bit offsets, which
 * plain C99 leaves out. */
#define _POSIX_C_SOURCE 200809L
#define _FILE_OFFSET_BITS 64
#ifdef __APPLE__
#define _DARWIN_C_SOURCE
#endif
#endif

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "tinf.h"
#include "xz.h"

#ifdef _WIN32
#include <direct.h>
#include <windows.h>
typedef wchar_t pchar;
#define P(s) L##s
#define PFMT L"%ls"
#define plen wcslen
#define pcpy wcscpy
#define pcat wcscat
#define pgetenv _wgetenv
#define pfopen _wfopen
#define prename _wrename
#define premove _wremove
#define pmkdir(d) _wmkdir(d)
#define SEP L'\\'
#define EXE L".exe"
typedef struct _stat64 pstat_t;
#define pstat _wstat64
#else
#include <unistd.h>
#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif
typedef char pchar;
#define P(s) s
#define PFMT "%s"
#define plen strlen
#define pcpy strcpy
#define pcat strcat
#define pgetenv getenv
#define pfopen fopen
#define prename rename
#define premove remove
#define pmkdir(d) mkdir(d, 0755)
#define SEP '/'
#define EXE ""
typedef struct stat pstat_t;
#define pstat stat
#endif

#define TRAILER 40
#define PATH_MAX_CHARS 4096

static void fail(const char *what) {
    fprintf(stderr, "error: this packed program can't start: %s\n", what);
    exit(2);
}

static uint64_t le64(const unsigned char *b) {
    uint64_t v = 0;
    for (int i = 7; i >= 0; i--) {
        v = (v << 8) | b[i];
    }
    return v;
}

/* A 64-bit FNV-1a hash, as `kavach_runtime_unpack::hash` makes it. */
static uint64_t fnv1a(const unsigned char *b, size_t n) {
    uint64_t h = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < n; i++) {
        h = (h ^ b[i]) * 0x100000001b3ull;
    }
    return h;
}

/* The path of this program's own file. */
static void own_path(pchar *out) {
#ifdef _WIN32
    DWORD n = GetModuleFileNameW(NULL, out, PATH_MAX_CHARS);
    if (n == 0 || n >= PATH_MAX_CHARS) fail("can't find its own file");
#elif defined(__APPLE__)
    char raw[PATH_MAX_CHARS];
    uint32_t size = sizeof raw;
    if (_NSGetExecutablePath(raw, &size) != 0 || !realpath(raw, out)) {
        fail("can't find its own file");
    }
#else
    ssize_t n = readlink("/proc/self/exe", out, PATH_MAX_CHARS - 1);
    if (n <= 0) fail("can't find its own file");
    out[n] = 0;
#endif
}

/* The cache folder of spec 10.5, made if missing. */
static void cache_dir(pchar *out) {
    const pchar *set = pgetenv(P("KAVACH_UNPACKED"));
    if (set && *set) {
        pcpy(out, set);
    } else {
#ifdef _WIN32
        const pchar *base = pgetenv(P("LOCALAPPDATA"));
        if (!base || !*base) fail("no cache folder: set KAVACH_UNPACKED");
        pcpy(out, base);
        pcat(out, P("\\Kavach\\unpacked"));
#else
        const char *xdg = getenv("XDG_CACHE_HOME");
        const char *home = getenv("HOME");
#ifdef __APPLE__
        xdg = NULL;
#endif
        if (xdg && *xdg) {
            pcpy(out, xdg);
            pcat(out, "/kavach/unpacked");
        } else if (home && *home) {
            pcpy(out, home);
#ifdef __APPLE__
            pcat(out, "/Library/Caches/kavach/unpacked");
#else
            pcat(out, "/.cache/kavach/unpacked");
#endif
        } else {
            fail("no cache folder: set KAVACH_UNPACKED");
        }
#endif
    }
    if (plen(out) > PATH_MAX_CHARS - 64) fail("the cache folder's path is too long");
    /* Each folder on the way, then the folder itself. */
    for (size_t i = 1; out[i]; i++) {
        if (out[i] == '/' || out[i] == '\\') {
            pchar keep = out[i];
            out[i] = 0;
            pmkdir(out);
            out[i] = keep;
        }
    }
    pmkdir(out);
}

/* Unpacks `packed` into `out`, `len` bytes, by `method`: 1 if right. */
static int unpack(int method, const unsigned char *packed, size_t packed_len,
                  unsigned char *out, size_t len) {
    if (method == 1) {
        unsigned int got = (unsigned int)len;
        return tinf_uncompress(out, &got, packed, (unsigned int)packed_len) == TINF_OK
            && got == len;
    }
    if (method == 2) {
        xz_crc32_init();
        struct xz_dec *s = xz_dec_init(XZ_SINGLE, 0);
        if (!s) return 0;
        struct xz_buf b = {packed, 0, packed_len, out, 0, len};
        enum xz_ret r = xz_dec_run(s, &b);
        xz_dec_end(s);
        return r == XZ_STREAM_END && b.out_pos == len;
    }
    return 0;
}

/* Reads `n` bytes at `at` of `f` into a new buffer. */
static unsigned char *read_at(FILE *f, long long at, size_t n) {
    unsigned char *b = malloc(n ? n : 1);
    if (!b) fail("out of memory");
#ifdef _WIN32
    if (_fseeki64(f, at, SEEK_SET) != 0) fail("can't read its own file");
#else
    if (fseeko(f, (off_t)at, SEEK_SET) != 0) fail("can't read its own file");
#endif
    if (fread(b, 1, n, f) != n) fail("can't read its own file");
    return b;
}

/* Makes the unpacked copy `copy` from the program packed in `f`. */
static void make_copy(FILE *f, long long end, const unsigned char *t, const pchar *copy) {
    int method = (int)le64(t + 8);
    uint64_t packed_len = le64(t + 16), len = le64(t + 24), hash = le64(t + 32);
    if (packed_len > (uint64_t)end || len > ((uint64_t)1 << 40)) fail("its trailer is damaged");
    unsigned char *packed = read_at(f, end - (long long)packed_len, (size_t)packed_len);
    unsigned char *program = malloc(len ? (size_t)len : 1);
    if (!program) fail("out of memory");
    if (!unpack(method, packed, (size_t)packed_len, program, (size_t)len)
        || fnv1a(program, (size_t)len) != hash) {
        fail("the packed program is damaged");
    }
    free(packed);
    /* Written whole under a name of its own, then renamed: a program
     * started at the same moment never runs half a file. */
    pchar part[PATH_MAX_CHARS];
    pcpy(part, copy);
#ifdef _WIN32
    swprintf(part + plen(part), 32, L".part%lu", (unsigned long)GetCurrentProcessId());
#else
    snprintf(part + plen(part), 32, ".part%ld", (long)getpid());
#endif
    FILE *out = pfopen(part, P("wb"));
    if (!out) fail("can't write into the cache folder");
    int ok = fwrite(program, 1, (size_t)len, out) == len;
    ok = (fclose(out) == 0) && ok;
    free(program);
    if (!ok) fail("can't write into the cache folder");
#ifndef _WIN32
    chmod(part, 0755);
#endif
    if (prename(part, copy) != 0) {
        /* Another start renamed its copy first: the same bytes. */
        premove(part);
    }
}

/* Runs the copy in place of the packed program. */
static int run(const pchar *copy, char **argv) {
#ifdef _WIN32
    (void)argv;
    SetConsoleCtrlHandler(NULL, TRUE);
    STARTUPINFOW si;
    PROCESS_INFORMATION pi;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    /* The program sees the command line this one was started with. */
    wchar_t *line = _wcsdup(GetCommandLineW());
    if (!line || !CreateProcessW(copy, line, NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi)) {
        fail("can't run the unpacked program");
    }
    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 1;
    GetExitCodeProcess(pi.hProcess, &code);
    ExitProcess(code);
#else
    execv(copy, argv);
    fail("can't run the unpacked program");
#endif
    return 2;
}

int main(int argc, char **argv) {
    (void)argc;
    pchar me[PATH_MAX_CHARS];
    own_path(me);
    FILE *f = pfopen(me, P("rb"));
    if (!f) fail("can't read its own file");
#ifdef _WIN32
    _fseeki64(f, 0, SEEK_END);
    long long size = _ftelli64(f);
#else
    fseeko(f, 0, SEEK_END);
    long long size = (long long)ftello(f);
#endif
    if (size < TRAILER) fail("no program is packed in it");
    unsigned char *t = read_at(f, size - TRAILER, TRAILER);
    if (memcmp(t, "KVPACK01", 8) != 0) fail("no program is packed in it");

    pchar copy[PATH_MAX_CHARS];
    cache_dir(copy);
    size_t n = plen(copy);
    copy[n++] = SEP;
#ifdef _WIN32
    swprintf(copy + n, 64, L"%016llx-%llu" EXE, (unsigned long long)le64(t + 32),
             (unsigned long long)le64(t + 24));
#else
    snprintf(copy + n, 64, "%016llx-%llu", (unsigned long long)le64(t + 32),
             (unsigned long long)le64(t + 24));
#endif
    pstat_t st;
    if (pstat(copy, &st) != 0 || (uint64_t)st.st_size != le64(t + 24)) {
        make_copy(f, size - TRAILER, t, copy);
    }
    fclose(f);
    return run(copy, argv);
}
