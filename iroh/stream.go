package iroh

import "io"

type Stream struct {
	handle int64
}

func (s *Stream) Read(
	p []byte,
) (int, error) {

	return 0, io.EOF
}

func (s *Stream) Write(
	p []byte,
) (int, error) {

	return len(p), nil
}
