package acme

import "fmt"

type Inventory struct {
	Count int
}

func (i *Inventory) Restock() {
	i.Count += 10
	fmt.Println("restocked")

func (i *Inventory) Drain() {
	i.Count = 0
}
