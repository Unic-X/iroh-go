package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"

// Endpoint Builder
type EndpointBuilder struct {
	ptr *C.EndpointBuilder_t
}

func NewEndpointBuilder() *EndpointBuilder {
	return &EndpointBuilder{
		ptr: C.endpoint_builder_new(),
	}
}

func (b *EndpointBuilder) Free() {
	C.endpoint_builder_free(b.ptr)
}

func (b *EndpointBuilder) N0() {
	C.apply_n0(b.ptr)
}

func (b *EndpointBuilder) Minimal() {
	C.apply_minimal(b.ptr)
}

func (b *EndpointBuilder) ApplyN0DisableRelay() {
	C.apply_n0_disable_relay(b.ptr)
}

func (b *EndpointBuilder) ApplyPreset(p Preset) {
	cp := C.Preset_t(p)
	C.apply_preset(&cp, b.ptr)
}

func (b *EndpointBuilder) BindAddr(addr string) error {
	vec := ToVec(addr)
	defer FreeVec(vec)

	res := C.bind_addr(b.ptr, vec)

	return ResultVoid(res)
}

func (b *EndpointBuilder) SecretKey(key []byte) error {
	key_vec := ToVec(key)

	defer FreeVec(key_vec)
	res := C.secret_key(b.ptr, key_vec)

	return ResultVoid(res)
}

// Returns Void
func (b *EndpointBuilder) RelayMode(mode RelayMode) {

	// Cast Type from uint8 to
	C.relay_mode(b.ptr, C.RelayModeFFI_t(mode))
}
