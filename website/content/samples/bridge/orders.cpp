#include <iostream>
#include <map>
#include <numeric>
#include <stdexcept>
#include <string>
#include <vector>

struct Order {
    std::string item;
    long qty;
    long cents;

    long cost() const { return qty * cents; }
};

struct BadQty : std::runtime_error {
    std::string item;
    explicit BadQty(const std::string& i) : std::runtime_error("no quantity for " + i), item(i) {}
};

long total(const std::vector<Order>& orders) {
    for (const auto& o : orders) {
        if (o.qty <= 0) throw BadQty(o.item);
    }
    return std::accumulate(orders.begin(), orders.end(), 0L,
                           [](long sum, const Order& o) { return sum + o.cost(); });
}

int main() {
    const std::map<std::string, long> codes{{"SPRING", 10}, {"VIP", 25}};
    const std::vector<Order> orders{{"pen", 3, 150}, {"cup", 1, 900}};

    long sum;
    try {
        sum = total(orders);
    } catch (const BadQty&) {
        sum = 0;
    }
    auto found = codes.find("SPRING");
    long off = found != codes.end() ? found->second : 0;
    std::cout << "total " << sum << ", with " << off << "% off " << sum * (100 - off) / 100 << "\n";
    auto winter = codes.find("WINTER");
    std::cout << "WINTER gives " << (winter != codes.end() ? winter->second : 0) << "%\n";

    const std::vector<Order> broken{{"kite", 0, 500}};
    try {
        long t = total(broken);
        std::cout << "total " << t << "\n";
    } catch (const BadQty& e) {
        std::cout << "refused: no quantity for " << e.item << "\n";
    }
}
