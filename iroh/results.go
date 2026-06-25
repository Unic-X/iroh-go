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
	"fmt"
	"unsafe"
)

type Result[T any] struct {
	Value T
	Err   error
}

// Error Handling and Result Handling
func ErrorFromC(err C.IrohError_t) error {
	msg := BytesToGo[string](err.message)

	// Free error message buffer if ownership was transferred.
	FreeVec(err.message)

	return errors.New(msg)
}

func ResultVoid(res C.IrohResult_void_t) error {
	if res.tag == C.IROH_RESULT_TAG_OK {
		return nil
	}

	return ErrorFromC(res.error._1)
}

func ResultValue[T any](
	value T,
	errTuple C.Tuple2_bool_IrohError_t,
) (*T, error) {
	if errTuple._0 {
		msg := C.GoStringN(
			(*C.char)(unsafe.Pointer(errTuple._1.message.ptr)),
			C.int(errTuple._1.message.len),
		)
		return nil, fmt.Errorf("%s", msg)
	}

	return &value, nil
}
