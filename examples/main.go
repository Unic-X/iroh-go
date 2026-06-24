package main

import (
	"fmt"
	"go-iroh/iroh"
)

func main() {
	builder := iroh.NewEndpointBuilder()
	defer builder.Free()

	builder.ApplyPreset(iroh.PresetN0)

	ep, err := builder.BindEndpoint()

	if err != nil {
		fmt.Println("Gaya baba")
	}

	ep.BoundSockets()

	fmt.Println("hihihahah")
}
