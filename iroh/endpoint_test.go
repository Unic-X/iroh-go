package iroh

import "testing"

func TestNewEndpoint(t *testing.T) {

	e, err := NewEndpoint()

	if err != nil {
		t.Fatal(err)
	}

	if e == nil {
		t.Fatal("nil endpoint")
	}

	err = e.Close()

	if err != nil {
		t.Fatal(err)
	}

	if e == nil {
		t.Fatal("nil endpoint")
	}
}
