package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"

type Connection struct {
	ptr *C.Connection_t
}

type BiStream struct {
	ptr *C.BiStream_t
}

type SendStream struct {
	ptr *C.SendStream_t
}

type RecvStream struct {
	ptr *C.RecvStream_t
}

func (c *Connection) Alpn() []byte {
	return BytesToGo[[]byte](C.connection_alpn(c.ptr))
}

func (c *Connection) OpenUni() (*SendStream, error) {
	res := C.connection_open_uni(c.ptr)
	return ResultValue(
		SendStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connection) AcceptUni() (*RecvStream, error) {
	res := C.connection_accept_uni(c.ptr)
	return ResultValue(
		RecvStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connection) OpenBi() (*BiStream, error) {
	res := C.connection_open_bi(c.ptr)
	return ResultValue(
		BiStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

func (c *Connection) AcceptBi() (*BiStream, error) {
	res := C.connection_accept_bi(c.ptr)
	return ResultValue(
		BiStream{
			ptr: res.value._1,
		},
		res.error,
	)
}

// Will this hack of BytesToGo work?
func (c *Connection) ReadDatagram() (*[]byte, error) {
	res := C.connection_read_datagram(c.ptr)
	if res.tag == C.IROH_RESULT_TAG_ERROR {
		return nil, ErrorFromC(res.error._1)
	}

	bytes := BytesToGo[[]byte](res.value._1)
	return &bytes, nil
}
