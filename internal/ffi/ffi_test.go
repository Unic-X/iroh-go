package ffi

import "testing"

func TestEndpointNew(t *testing.T) {

	h := EndpointNew()

	if h <= 0 {
		t.Fatal("invalid handle")
	}

	EndpointFree(h)
}
