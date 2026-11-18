// Many small values made together and dropped together: 100,000 lines
// split into words (1.1 million of them), ten times over; each round's
// words come from one std::pmr::monotonic_buffer_resource.
#include <cstdio>
#include <string>
#include <memory_resource>
#include <vector>

static std::pmr::vector<std::pmr::string> split(const std::string& s, char c, std::pmr::memory_resource* m) {
    std::pmr::vector<std::pmr::string> out(m);
    size_t start = 0;
    for (size_t i = 0; i <= s.size(); i++) {
        if (i == s.size() || s[i] == c) {
            out.emplace_back(s.data() + start, i - start);
            start = i + 1;
        }
    }
    return out;
}

static long long letters(const std::vector<std::string>& lines) {
    std::pmr::monotonic_buffer_resource arena;
    long long n = 0;
    for (const auto& line : lines) {
        for (const auto& word : split(line, ' ', &arena)) {
            n += (long long)word.size();
        }
    }
    return n;
}

int main() {
    std::vector<std::string> lines;
    for (int i = 0; i < 100000; i++) {
        lines.push_back("let x" + std::to_string(i) + " = y" + std::to_string(i) + " + " +
                        std::to_string(i % 97) + " * ( z - " + std::to_string(i % 7) + " )");
    }
    long long total = 0;
    for (int r = 0; r < 10; r++) {
        total += letters(lines);
    }
    std::printf("%lld\n", total);
}
