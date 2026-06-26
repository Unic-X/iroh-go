package main

import (
	"fmt"
	"go-iroh/iroh"
)

func main() {
	builder := iroh.NewEndpointBuilder()
	defer builder.Free()

	builder.ApplyPreset(iroh.PresetN0)

	ep1, _ := builder.BindEndpoint()

	for _, socket := range ep1.BoundSockets() {
		fmt.Println(socket)
	}

}
