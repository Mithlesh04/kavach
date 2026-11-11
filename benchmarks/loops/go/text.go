// Text: build 1 million words into one text, then split and count them.
package main

import (
	"fmt"
	"strconv"
	"strings"
)

func main() {
	var parts []string
	for i := int64(0); i < 1000000; i++ {
		parts = append(parts, "word"+strconv.FormatInt(i%50, 10))
	}
	var text strings.Builder
	for i, p := range parts {
		if i > 0 {
			text.WriteString(" ")
		}
		text.WriteString(p)
	}
	words := strings.Split(text.String(), " ")
	var longWords int64
	for _, w := range words {
		if len(w) > 5 {
			longWords++
		}
	}
	fmt.Println(longWords)
}
