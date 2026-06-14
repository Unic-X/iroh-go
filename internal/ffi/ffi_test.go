package ffi

import "testing"

func TestEndpointNew(t *testing.T) {

	h := EndpointNew()

	if h <= 0 {
		t.Fatal("invalid handle")
	}

	EndpointFree(h)
}

func TestEndpointNodeId(t *testing.T) {
	h := EndpointNew()
	if h <= 0 {
		t.Fatal("invalid handle")
	}
	defer EndpointFree(h)

	nodeId, err := EndpointNodeId(h)
	if err != nil {
		t.Fatal(err)
	}
	t.Log(nodeId)
}
