// Checksums: CRC32C over 64 MB, 64 times, with the processor's
// instruction, as a C++ storage engine does.
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <vector>
#include <nmmintrin.h>

// GCC and Clang need the attribute to use the instruction; MSVC doesn't.
#if defined(__GNUC__)
__attribute__((target("sse4.2")))
#endif
static uint32_t crc32c(const std::vector<uint8_t>& data) {
    uint64_t crc = ~0u;
    size_t i = 0;
    for (; i + 8 <= data.size(); i += 8) {
        uint64_t word;
        std::memcpy(&word, data.data() + i, 8);
        crc = _mm_crc32_u64(crc, word);
    }
    uint32_t c = (uint32_t)crc;
    for (; i < data.size(); i++) c = _mm_crc32_u8(c, data[i]);
    return ~c;
}

int main() {
    std::vector<uint8_t> data;
    data.reserve(67108864);
    for (long long i = 0; i < 67108864; i++) data.push_back((uint8_t)(i % 251));
    std::printf("%u\n", crc32c(data));
}
