package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"

type Endpoint struct {
	ptr *C.Endpoint_t
}

type ProtocolHandler struct {
	Alpn       []byte
	OnAccept   func(*Connection) bool
	OnShutdown func()
}

type EndpointOptions struct {
	Preset    Preset
	BindAddr  []byte
	SecretKey []byte
	Alpns     [][]byte
	RelayMode RelayMode
	Protocols []ProtocolHandler
}

type EndpointId struct {
	key []byte
}

func BindEndpoint(epOptions EndpointOptions) Endpoint {
	options := C.EndpointOptions_t{
		preset:     C.Preset_t(epOptions.Preset),
		bind_addr:  ToVec(epOptions.BindAddr),
		secret_key: ToVec(epOptions.SecretKey),
		alpns:      ToVecVec(epOptions.Alpns),
		relay_mode: C.RelayModeFFI_t(epOptions.RelayMode),
	}

	ep := C.endpoint_bind(&options)

	return Endpoint{
		ptr: ep.value._1, // Error handling needs to be done
	}
}

func (e *Endpoint) Id() EndpointId {
	id := C.endpoint_id(e.ptr)

	key := make([]byte, 32)

	for _, item := range id.key.idx {
		key = append(key, byte(item))
	}

	return EndpointId{
		key: key,
	}
}

func (e *Endpoint) BoundSockets() []string {
	sockets := C.endpoint_bound_sockets(e.ptr)
	return VecStringToSlice(sockets)
}
