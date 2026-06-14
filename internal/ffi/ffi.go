package ffi

/*
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include <stdlib.h>
#include <stdint.h>
#include "../../iroh-rs/include/iroh.h"
*/
import "C"
import (
	"go-iroh/internal/iroherr"
	"unsafe"
)

type EndpointHandle int64
type ConnectionHandle int64

func EndpointNew() EndpointHandle {
	return EndpointHandle(C.iroh_endpoint_new())
}

func EndpointFree(id EndpointHandle) error {
	ok := bool(C.iroh_endpoint_free(C.int64_t(id)))
	if !ok {
		return iroherr.ErrEndpointFree
	}
	return nil
}

func Connect(endpoint EndpointHandle, nodeID string) (ConnectionHandle, error) {
	cNodeID := C.CString(nodeID)
	defer C.free(unsafe.Pointer(cNodeID))

	conn := ConnectionHandle(C.iroh_connect(
		C.int64_t(endpoint),
		cNodeID,
	))

	if conn <= 0 {
		return 0, iroherr.ErrConnect
	}

	return conn, nil
}

func EndpointNodeId(endpoint EndpointHandle) (string, error) {
	cNodeId := C.iroh_endpoint_node_id(C.int64_t(endpoint))
	if cNodeId == nil {
		return "", iroherr.ErrEndpointNodeId
	}
	defer C.free(unsafe.Pointer(cNodeId))
	return C.GoString(cNodeId), nil
}

func ConnectionClose(conn ConnectionHandle) error {
	ok := bool(C.iroh_connection_close(C.int64_t(conn)))
	if !ok {
		return iroherr.ErrConnectionClose
	}
	return nil
}
