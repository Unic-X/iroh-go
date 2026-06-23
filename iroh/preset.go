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
	RelayModeDisabled RelayMode = 0
	RelayModeDefault
	RelayModeStaging
)
