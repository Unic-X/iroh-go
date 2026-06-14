package iroh

import (
	"runtime"

	"go-iroh/internal/ffi"
)

type Endpoint struct {
	handle ffi.EndpointHandle
}

func NewEndpoint() (*Endpoint, error) {
	h := ffi.EndpointNew()

	e := &Endpoint{
		handle: h,
	}

	runtime.AddCleanup(e, func(h ffi.EndpointHandle) {
		ffi.EndpointFree(h)
	}, e.handle)

	return e, nil
}

func (e *Endpoint) Close() error {
	if e.handle == 0 {
		return nil
	}

	ffi.EndpointFree(e.handle)

	e.handle = 0

	return nil
}

func (e *Endpoint) NodeId() (string, error) {
	return ffi.EndpointNodeId(e.handle)
}
