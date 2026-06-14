package iroh

import (
	"runtime"

	"go-iroh/internal/ffi"
)

type Connection struct {
	handle ffi.ConnectionHandle
}

func (e *Endpoint) Connect(
	nodeID string,
) (*Connection, error) {

	h, err := ffi.Connect(
		e.handle,
		nodeID,
	)

	if err != nil {
		return nil, err
	}

	c := &Connection{
		handle: h,
	}

	runtime.AddCleanup(c, func(h ffi.ConnectionHandle) {
		ffi.ConnectionClose(h)
	}, c.handle)

	return c, nil
}

func (c *Connection) Close() {
	if c.handle == 0 {
		return
	}

	ffi.ConnectionClose(
		c.handle,
	)

	c.handle = 0
}
