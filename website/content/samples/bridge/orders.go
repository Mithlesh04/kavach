package main

import "fmt"

type Order struct {
	Item  string
	Qty   int
	Cents int
}

func (o Order) Cost() int {
	return o.Qty * o.Cents
}

type BadQty struct {
	Item string
}

func (e BadQty) Error() string {
	return "no quantity for " + e.Item
}

func total(orders []Order) (int, error) {
	sum := 0
	for _, o := range orders {
		if o.Qty <= 0 {
			return 0, BadQty{Item: o.Item}
		}
		sum += o.Cost()
	}
	return sum, nil
}

func main() {
	codes := map[string]int{"SPRING": 10, "VIP": 25}
	orders := []Order{{"pen", 3, 150}, {"cup", 1, 900}}

	sum, err := total(orders)
	if err != nil {
		sum = 0
	}
	off := codes["SPRING"]
	fmt.Printf("total %d, with %d%% off %d\n", sum, off, sum*(100-off)/100)
	fmt.Printf("WINTER gives %d%%\n", codes["WINTER"])

	broken := []Order{{"kite", 0, 500}}
	if t, err := total(broken); err != nil {
		if bad, ok := err.(BadQty); ok {
			fmt.Printf("refused: no quantity for %s\n", bad.Item)
		}
	} else {
		fmt.Printf("total %d\n", t)
	}
}
