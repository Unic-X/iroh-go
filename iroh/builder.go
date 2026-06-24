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

// Replay the n0 production preset (relays + discovery + crypto provider).
func (b *EndpointBuilder) N0() {
	C.apply_n0(b.ptr)
}

// Replay the minimal preset (crypto provider only, no external deps).
func (b *EndpointBuilder) Minimal() {
	C.apply_minimal(b.ptr)
}

// Replay the n0 preset with relays disabled.
func (b *EndpointBuilder) ApplyN0DisableRelay() {
	C.apply_n0_disable_relay(b.ptr)
}

// Set the advertised ALPNs.
func (b *EndpointBuilder) SetAlpns(alpns [][]byte) {
	cAlpns := ToVecVec(alpns)
	defer FreeVecVec(cAlpns)
	C.set_alpns(b.ptr, cAlpns)
}

// / Set the endpoint secret key (32 bytes).
func (b *EndpointBuilder) SecretKey(key []byte) error {
	key_vec := ToVec(key)

	defer FreeVec(key_vec)
	res := C.secret_key(b.ptr, key_vec)

	return ResultVoid(res)
}

// Set the relay mode.
// Returns Void
func (b *EndpointBuilder) RelayMode(mode RelayMode) {
	// Cast Type from uint8 to C type
	C.relay_mode(b.ptr, C.RelayModeFFI_t(mode))
}

// Set the address the endpoint binds to (`host:port`).
func (b *EndpointBuilder) BindAddr(addr string) error {
	vec := ToVec(addr)
	defer FreeVec(vec)
	res := C.bind_addr(b.ptr, vec)
	return ResultVoid(res)
}

func (b *EndpointBuilder) ApplyPreset(p Preset) {
	cp := C.Preset_t(p)
	C.apply_preset(&cp, b.ptr)
}
