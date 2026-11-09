// Float arithmetic in a loop: 200 million rounds.
#include <cstdio>

int main() {
    double x = 0.0;
    double y = 1.0;
    for (long long i = 0; i < 200000000; i++) {
        x = x * 0.999999 + y;
        y = y * 1.0000001 - 0.0000001;
    }
    std::printf("%s\n", x > 0.0 ? "true" : "false");
}
