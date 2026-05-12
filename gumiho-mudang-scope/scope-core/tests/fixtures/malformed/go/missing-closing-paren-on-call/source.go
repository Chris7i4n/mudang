package acme

import "fmt"

func Greet(name string) string {
	return fmt.Sprintf("Hello, %s — welcome to %s", name, "Acme"
}

func Farewell(name string) string {
	return fmt.Sprintf("Bye, %s", name)
}
