package iroh

type Preset int

const (
	PresetNone Preset = iota
	PresetN0
	PresetMinimal
	PresetN0DisableRelay
)

type RelayMode uint8

const (
	RelayModeDisabled RelayMode = iota
	RelayModeDefault
	RelayModeStaging
)

type Side uint8

const (
	ClientSide Side = iota
	ServerSide
)
