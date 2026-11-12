# 10.10 A library for other languages

`kavach build --lib` compiles a library package (`src/lib.kav`, 10.3) into a shared library that other languages load through C: Node.js (Node-API, `ffi`), Java (JNI, the FFM API), Go (cgo), C# (P/Invoke), Python (`ctypes`), C and C++ ([decision 0026](../../decisions/0026-a-library-for-other-languages.md)).

```kavach
// src/lib.kav of the package `geometry`
pub fn area(width: Float, height: Float) -> Float {
    width * height
}
```

`kavach build --lib` writes `.kavach/lib/geometry.dll` on Windows, `libgeometry.so` on Linux or `libgeometry.dylib` on macOS, and the header `geometry.h`:

```c
#include "geometry.h"

double a = geometry_area(2.0, 3.5);     // 7.0
```

- **What is exported.** Every `pub fn` of `src/lib.kav` whose parameters and result are numbers, `Bool`, `Ptr` (10.7) or `String`, under the C name `<package>_<fn>`; a `-` in the package name becomes `_`. Other `pub` functions are listed as not exported, with the reason. They stay usable from Kavach.
- **Text.** A `String` parameter is a `const char*`: UTF-8 ending in a zero byte, copied when the call starts; a null pointer or bytes that aren't UTF-8 panic. A `String` result is a new `char*` of the same form, which the caller frees with `<package>_free`, a function every library exports (C's `free` may belong to another C library than the one Kavach's library uses).
- **Errors.** A function that can fail (`-> T ! E`) returns an `int32_t` instead: 0 when it succeeded, and then its value is written to one more parameter, `T* result` (left out when there is no value; `result` may be null when the caller doesn't need it). When it failed, the number is the error's variant, counting from 1 in the order the union declares them, or 1 for an error that isn't a union. The header names each number, and `<package>_last_error()` gives the error as text, as Kavach prints it (`TooOld(age: 200)`), to free with `<package>_free` (null before any error).

  ```kavach
  pub type AgeError = Negative | TooOld(age: Int)

  pub fn check_age(age: Int) -> Int ! AgeError {
      if age < 0 { fail AgeError.Negative }
      if age > 150 { fail AgeError.TooOld(age: age) }
      age
  }
  ```

  ```c
  #define PEOPLE_AGE_ERROR_NEGATIVE 1
  #define PEOPLE_AGE_ERROR_TOO_OLD 2
  int32_t people_check_age(int64_t age, int64_t* result);

  int64_t age;
  if (people_check_age(200, &age) == PEOPLE_AGE_ERROR_TOO_OLD) {
      char* why = people_last_error();     // "TooOld(age: 200)"
      people_free(why);
  }
  ```
- **The header** declares each exported function with C types: `int64_t` for `Int`, `double` for `Float`, `bool` for `Bool`, `T*` for `Ptr<T>`, `void*` for a pointer to a record, and `const char*` and `char*` for text going in and out.
- **Calls** may come from any thread, and from several at once ([decision 0035](../../decisions/0035-library-calls-overlap.md)). Each call computes at the same time as the others until its first effect another call could see (printing, logging, files, the network, tasks, channels, a panic); there it waits for the library's lock, and keeps it until it returns. So what different calls print and do never interleaves. A C callback that calls back in from the same call goes on instead of waiting. `<package>_last_error()` gives the last error of the calling thread. C functions the library calls (10.7) may run in several calls at once. The first call sets up Kavach's runtime; what an exported function prints is written out before it returns. A panic prints its message and ends the process, as in a program (5.8).
- **Native code only:** the library is compiled code, like the executables of `kavach build`.
