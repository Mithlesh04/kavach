// Text: build 1 million words into one text, then split and count them.
#include <cstdio>
#include <string>
#include <vector>

int main() {
    std::vector<std::string> parts;
    for (long long i = 0; i < 1000000; i++) {
        parts.push_back("word" + std::to_string(i % 50));
    }
    std::string text;
    for (size_t i = 0; i < parts.size(); i++) {
        if (i > 0) text += " ";
        text += parts[i];
    }
    std::vector<std::string> words;
    size_t start = 0;
    while (true) {
        size_t end = text.find(' ', start);
        words.push_back(text.substr(start, end - start));
        if (end == std::string::npos) break;
        start = end + 1;
    }
    long long long_words = 0;
    for (const std::string& w : words) {
        if (w.size() > 5) long_words++;
    }
    std::printf("%lld\n", long_words);
}
