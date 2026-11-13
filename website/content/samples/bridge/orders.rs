use std::collections::HashMap;

struct Order {
    item: String,
    qty: i64,
    cents: i64,
}

impl Order {
    fn cost(&self) -> i64 {
        self.qty * self.cents
    }
}

enum OrderError {
    BadQty { item: String },
}

fn total(orders: &[Order]) -> Result<i64, OrderError> {
    for o in orders {
        if o.qty <= 0 {
            return Err(OrderError::BadQty { item: o.item.clone() });
        }
    }
    Ok(orders.iter().map(|o| o.cost()).sum())
}

fn main() {
    let codes = HashMap::from([("SPRING", 10), ("VIP", 25)]);
    let orders = vec![
        Order { item: "pen".to_string(), qty: 3, cents: 150 },
        Order { item: "cup".to_string(), qty: 1, cents: 900 },
    ];

    let sum = total(&orders).unwrap_or(0);
    let off = *codes.get("SPRING").unwrap_or(&0);
    println!("total {sum}, with {off}% off {}", sum * (100 - off) / 100);
    println!("WINTER gives {}%", codes.get("WINTER").unwrap_or(&0));

    let broken = vec![Order { item: "kite".to_string(), qty: 0, cents: 500 }];
    match total(&broken) {
        Ok(t) => println!("total {t}"),
        Err(OrderError::BadQty { item }) => println!("refused: no quantity for {item}"),
    }
}
