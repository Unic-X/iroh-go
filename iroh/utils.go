package iroh

/*
#include <stdlib.h>
#include <stddef.h>
#cgo linux LDFLAGS: -L${SRCDIR}/../../iroh-rs/target/release -lgo_iroh
#include "../iroh-rs/include/iroh.h"
*/
import "C"
import (
	"unsafe"
)

func FreeVec(v C.Vec_uint8_t) {
	if v.ptr != nil {
		C.free(unsafe.Pointer(v.ptr))
	}
}

// Generic impl for Iterable
// Strict constraint for now
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

func ToVecVec[T Iterable](items []T) C.Vec_Vec_uint8_t {
	if len(items) == 0 {
		return C.Vec_Vec_uint8_t{}
	}

	elemSize := C.size_t(unsafe.Sizeof(C.Vec_Vec_uint8_t{}))
	totalSize := C.size_t(len(items)) * elemSize

	cArrayPtr := (*C.Vec_uint8_t)(C.malloc(totalSize))

	cSlice := unsafe.Slice(cArrayPtr, len(items))

	for i, item := range items {
		cSlice[i] = ToVec(item)
	}

	return C.Vec_Vec_uint8_t{
		ptr: cArrayPtr,
		len: C.size_t(len(items)),
		cap: C.size_t(len(items)),
	}

}

func VecStringToSlice(v C.Vec_Vec_uint8_t) []string {
	if v.ptr == nil || v.len == 0 {
		return nil
	}

	strs := unsafe.Slice(
		(*C.Vec_uint8_t)(unsafe.Pointer(v.ptr)),
		int(v.len),
	)

	result := make([]string, 0, int(v.len))

	for _, s := range strs {
		result = append(result, string(SliceFromC(s.ptr, s.len)))
	}

	return result
}

func FreeVecVec(v C.Vec_Vec_uint8_t) {
	if v.ptr == nil {
		return
	}

	cSlice := unsafe.Slice(v.ptr, int(v.len))

	for _, vec := range cSlice {
		FreeVec(vec)
	}

	C.free(unsafe.Pointer(v.ptr))
}

func BytesToGo[T Iterable](s C.Vec_uint8_t) T {
	if s.ptr == nil || s.len == 0 {
		var zero T
		return zero
	}

	bytes := unsafe.Slice(
		(*byte)(unsafe.Pointer(s.ptr)),
		int(s.len),
	)

	return T(bytes)
}

func SliceFromC[T any](ptr *T, len C.size_t) []T {
	if ptr == nil || len == 0 {
		return nil
	}

	return unsafe.Slice(ptr, int(len))
}
