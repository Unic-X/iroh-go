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

type EndpointAddr struct {
	ptr *C.EndpointAddr_t
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

type SecretKey struct {
	ptr *C.SecretKey_t
}

type RelayConfig struct {
	ptr *C.RelayConfig_t
}

// /// Bind a new endpoint with the given options.
func Bind(epOptions EndpointOptions) Endpoint {
	options := C.EndpointOptions_t{
		preset:     C.Preset_t(epOptions.Preset),
		bind_addr:  ToVec(epOptions.BindAddr),
		secret_key: ToVec(epOptions.SecretKey),
		alpns:      ToVecVec(epOptions.Alpns),
		relay_mode: C.RelayModeFFI_t(epOptions.RelayMode),
	}

	ep := C.endpoint_bind(
		&options,
	)

	return Endpoint{
		ptr: ep.value._1, // Error handling needs to be done
	}
}

// Returns the endpointId of this Endpoint
func (e *Endpoint) Id() EndpointId {
	id := C.endpoint_id(
		e.ptr,
	)
	key := make([]byte, 0, 32)

	for _, item := range id.key.idx {
		key = append(key, byte(item))
	}

	return EndpointId{
		key: key,
	}
}

func (e *Endpoint) BoundSockets() []string {
	sockets := C.endpoint_bound_sockets(
		e.ptr,
	)

	return VecStringToSlice(sockets)
}

func (e *Endpoint) Connect(addr EndpointAddr, alpn []byte) (*Connection, error) {
	res := C.endpoint_connect(
		e.ptr,
		addr.ptr,
		ToVec(alpn),
	)

	return ResultValue(
		Connection{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (e *Endpoint) Close() error {
	res := C.endpoint_close(e.ptr)

	return ResultVoid(res)
}

func (e *Endpoint) Free() {
	C.endpoint_free(e.ptr)
}

func (e *Endpoint) IsClosed() bool {
	res := C.endpoint_is_closed(
		e.ptr,
	)

	return bool(res)
}

func (e *Endpoint) SecretKey() SecretKey {
	return SecretKey{ptr: C.endpoint_secret_key(e.ptr)}
}

func (e *Endpoint) AddExternalAddr(addr string) error {
	caddr := ToVec(addr)
	defer FreeVec(caddr)

	res := C.endpoint_add_external_addr(e.ptr, caddr)

	return ResultVoid(res)
}

func (e *Endpoint) RemoveExternalAddr(addr string) (*bool, error) {
	caddr := ToVec(addr)
	defer FreeVec(caddr)

	res := C.endpoint_remove_external_addr(e.ptr, caddr)

	return ResultValue(bool(res.value._1), res.error)
}

func (e *Endpoint) InsertRelay(config *RelayConfig) error {
	res := C.endpoint_insert_relay(e.ptr, config.ptr)

	return ResultVoid(res)
}

func (e *Endpoint) RemoveRelay(url string) (*bool, error) {
	curl := ToVec(url)
	defer FreeVec(curl)

	res := C.endpoint_remove_relay(e.ptr, curl)

	return ResultValue(
		bool(res.value._1),
		res.error,
	)
}

func (e *Endpoint) AcceptNext() *Incoming {
	res := C.endpoint_accept_next(e.ptr)

	if !bool(res._0) {
		return nil
	}

	return &Incoming{
		ptr: res._1,
	}
}

func (e *Endpoint) ConnectPending(addr EndpointAddr, alpn []byte) (*Connecting, error) {

	res := C.endpoint_connect_pending(e.ptr, addr.ptr, ToVec(alpn))

	return ResultValue(
		Connecting{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (b *Endpoint) Addr() EndpointAddr {
	res := C.endpoint_addr(b.ptr)
	return EndpointAddr{
		ptr: res,
	}
}

func (b *Endpoint) Online() {
	C.endpoint_online(b.ptr)
}

// Accept

type IncomingLocalAddrKind uint8

const (
	Ip IncomingLocalAddrKind = iota
	Relay
	Custom
)

// / The local address that received an incoming connection.
type IncomingLocalAddr struct {
	kind IncomingLocalAddrKind
	addr string
	/// Relay path.
	url string
	/// Custom transport.
	description string
}

type Incoming struct {
	ptr *C.Incoming_t
}

func (i *Incoming) Accept() (*Accepting, error) {
	res := C.incoming_accept(i.ptr)

	return ResultValue(
		Accepting{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (i *Incoming) Refuse() error {
	res := C.incoming_refuse(i.ptr)

	return ResultVoid(res)
}

func (i *Incoming) Retry() error {
	res := C.incoming_retry(i.ptr)

	return ResultVoid(res)
}

func (i *Incoming) Ignore() error {
	res := C.incoming_ignore(i.ptr)

	return ResultVoid(res)
}

func (i *Incoming) LocalAddr() (*IncomingLocalAddr, error) {
	res := C.incoming_local_addr(i.ptr)

	return ResultValue(
		IncomingLocalAddr{
			kind:        IncomingLocalAddrKind(res.value._1.kind),
			addr:        BytesToGo[string](res.value._1.addr),
			url:         BytesToGo[string](res.value._1.url),
			description: BytesToGo[string](res.value._1.description),
		},
		res.error,
	)
}

// Accepting

type Accepting struct {
	ptr *C.Accepting_t
}

func (a *Accepting) Connect() (*Connection, error) {
	res := C.accepting_connect(a.ptr)

	return ResultValue(
		Connection{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (a *Accepting) Alpn() (*[]byte, error) {
	res := C.accepting_alpn(a.ptr)

	return ResultValue(
		BytesToGo[[]byte](res.value._1),
		res.error,
	)
}

//Connecting

type Connecting struct {
	ptr *C.Connecting_t
}

func (c *Connecting) Connect() (*Connection, error) {
	res := C.connecting_connect(c.ptr)

	return ResultValue(
		Connection{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connecting) Alpn() (*[]byte, error) {
	res := C.connecting_alpn(c.ptr)

	return ResultValue(
		BytesToGo[[]byte](res.value._1),
		res.error,
	)
}

func (c *Connecting) RemoteId() (*EndpointId, error) {
	res := C.connecting_remote_id(c.ptr)

	key := make([]byte, 0, 32)

	for _, item := range res.value._1.key.idx {
		key = append(key, byte(item))
	}

	return ResultValue(
		EndpointId{
			key: key,
		},
		res.error,
	)
}
