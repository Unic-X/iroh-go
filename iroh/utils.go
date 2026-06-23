package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"
import (
	"errors"
	"unsafe"
)

func FreeVec(v C.Vec_uint8_t) {
	if v.ptr != nil {
		C.free(unsafe.Pointer(v.ptr))
	}
}

// Generic impl for Iterable
type Iterable interface {
	string | ~[]byte
}

func ToVec[T Iterable](s T) C.Vec_uint8_t {
	if len(s) == 0 {
		return C.Vec_uint8_t{}
	}

	ptr := C.CBytes([]byte(s))

	return C.Vec_uint8_t{
		ptr: (*C.uint8_t)(ptr),
		len: C.size_t(len(s)),
		cap: C.size_t(len(s)),
	}
}

func VecToString(v C.Vec_uint8_t) string {
	if v.ptr == nil || v.len == 0 {
		return ""
	}

	bytes := unsafe.Slice(
		(*byte)(unsafe.Pointer(v.ptr)),
		int(v.len),
	)

	return string(bytes)
}

// Error Handling and Result Handling
func ErrorFromC(err C.IrohError_t) error {
	msg := VecToString(err.message)

	// Free error message buffer if ownership was transferred.
	FreeVec(err.message)

	return errors.New(msg)
}

type Result[T any] struct {
	Value T
	Err   error
}

func ResultVoid(res C.IrohResult_void_t) error {
	if res.tag == C.IROH_RESULT_TAG_OK {
		return nil
	}

	return ErrorFromC(res.error._1)
}

func ResultValue[T any](
	ok bool,
	value T,
	err C.IrohError_t,
) (T, error) {
	if ok {
		return value, nil
	}

	var zero T
	return zero, ErrorFromC(err)
}
