// A chain of builder methods, 10 million times.
#include <cstdio>

struct Transfer {
    long long from = 0, to = 0, amount = 0, fee = 0, note = 0;
    Transfer from_account(long long a) const {
        Transfer t = *this;
        t.from = a;
        return t;
    }
    Transfer to_account(long long a) const {
        Transfer t = *this;
        t.to = a;
        return t;
    }
    Transfer pay(long long n) const {
        Transfer t = *this;
        t.amount = n;
        return t;
    }
    Transfer with_fee(long long n) const {
        Transfer t = *this;
        t.fee = n;
        return t;
    }
    Transfer tagged(long long n) const {
        Transfer t = *this;
        t.note = n;
        return t;
    }
};

int main() {
    long long total = 0;
    for (long long i = 0; i < 10000000; i++) {
        Transfer t = Transfer{}
                         .from_account(i)
                         .to_account(i + 1)
                         .pay(i % 100)
                         .with_fee(i % 7)
                         .tagged(i % 3);
        total += t.amount + t.fee + t.note;
    }
    std::printf("%lld\n", total);
}
