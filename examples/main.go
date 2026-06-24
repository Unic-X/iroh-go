package main

import (
	"go-iroh/iroh"
)

func main() {
	builder := iroh.NewEndpointBuilder()
	defer builder.Free()

	builder.ApplyPreset(iroh.PresetN0)
}
