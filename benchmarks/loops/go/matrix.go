// Nested lists: multiply two 300 x 300 matrices (27 million multiply-adds).
package main

import "fmt"

func make2(n, seed int64) [][]float64 {
	var rows [][]float64
	for i := int64(0); i < n; i++ {
		var row []float64
		for j := int64(0); j < n; j++ {
			row = append(row, float64((i*seed+j)%10))
		}
		rows = append(rows, row)
	}
	return rows
}

func main() {
	n := int64(300)
	a, b := make2(n, 3), make2(n, 7)
	total := 0.0
	for i := int64(0); i < n; i++ {
		row := a[i]
		for j := int64(0); j < n; j++ {
			cell := 0.0
			for k := int64(0); k < n; k++ {
				cell += row[k] * b[k][j]
			}
			total += cell
		}
	}
	fmt.Printf("%.1f\n", total)
}
