package iroherr

import "errors"

var (
	ErrClosed = errors.New(
		"iroh: closed",
	)

	ErrConnect = errors.New(
		"iroh: connect failed",
	)

	ErrEndpointFree = errors.New(
		"iroh: endpoint free failed",
	)

	ErrConnectionClose = errors.New(
		"iroh: connection close failed",
	)

	ErrEndpointNodeId = errors.New(
		"iroh: endpoint node id failed",
	)
)
