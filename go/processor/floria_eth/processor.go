package floria_eth

import (
	"github.com/0xsoniclabs/tosca/go/processor/floria"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

func init() {
	// Register an ethereum compatible version of the geth processor.
	tosca.RegisterProcessorFactory("floria-eth", floriaEthereum)
}

func floriaEthereum(interpreter tosca.Interpreter) tosca.Processor {
	return &floria.Processor{
		Interpreter:        interpreter,
		EthereumCompatible: true,
	}
}
