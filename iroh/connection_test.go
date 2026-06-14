package iroh

import (
	"testing"
)

func TestConnection(t *testing.T) {
	e1, err := NewEndpoint()
	if err != nil {
		t.Fatal("failed to create endpoint")
	}
	defer e1.Close()

	e2, err := NewEndpoint()
	if err != nil {
		t.Fatal("failed to create endpoint")
	}
	defer e2.Close()

	nodeId, err := e2.NodeId()
	if err != nil {
		t.Fatal(err)
	}
	c, err := e1.Connect(nodeId)
	if err != nil {
		t.Fatal(err)
	}
	defer c.Close()
}
