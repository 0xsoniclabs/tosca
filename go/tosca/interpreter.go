// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package tosca

import "fmt"

//go:generate mockgen -source interpreter.go -destination interpreter_mock.go -package tosca

// Interpreter is a component capable of executing EVM byte-code. It is the main
// part of an EVM implementation, though a full EVM adds the ability to handle
// recursive contract calls and transaction handling.
// To obtain an Interpreter instance, client code should use GetInterpreter() provided
// by the registry file in this package.
type Interpreter interface {
	// Run executes the code provided by the parameters in the specified context
	// and returns the processing result. The resulting error is nil whenever the
	// code was correctly executed (even if the execution was aborted due do to
	// a code-internal issue). The error is not nil if some problem within the
	// interpreter caused the execution to fail to correctly process the provided
	// program. In such a case the result is undefined. During a call with an
	// unsupported Revision an ErrUnsupportedRevision Error is returned.
	// Interpreters are required to be thread-safe. Thus, multiple runs may be
	// conducted in parallel.
	Run(Parameters) (Result, error)
}

// Parameters summarizes the list of input parameters required for executing code.
type Parameters struct {
	BlockParameters
	TransactionParameters
	Context   RunContext
	Kind      CallKind
	Static    bool
	Depth     int
	Gas       Gas
	Recipient Address
	Sender    Address
	Input     Data
	Value     Value
	CodeHash  *Hash
	Code      Code
}

// BlockParameters contains information about the current block.
type BlockParameters struct {
	ChainID     Word
	BlockNumber int64
	Timestamp   int64
	Coinbase    Address
	GasLimit    Gas
	PrevRandao  Hash
	BaseFee     Value
	BlobBaseFee Value
	Revision    Revision
}

// TransactionParameters contains information about current transaction.
type TransactionParameters struct {
	Origin     Address
	GasPrice   Value
	BlobHashes []Hash
}

// RunContext provides an interface to access and manipulate state and transaction
// properties as needed by individual EVM instructions.
type RunContext interface {
	InterpreterContext

	Call(kind CallKind, parameter CallParameters) (CallResult, error)
}

// ProcessorContext is an interface to access and manipulate the state of the
// the world state in a transaction. All modifications on the world state are
// buffered in a transaction context, which can be snapshot and restored.
type ProcessorContext interface {
	InterpreterContext

	AccountExists(Address) bool
	CreateContract(Address)

	SetCode(Address, Code)
	SetNonce(Address, uint64)

	CreateSnapshot() Snapshot
	RestoreSnapshot(Snapshot)

	HasEmptyStorage(Address) bool

	GetLogs() []Log
}

// AccessStatus is an enum utilized to indicate cold and warm account or
// storage slot accesses.
type AccessStatus bool

const (
	ColdAccess AccessStatus = false
	WarmAccess AccessStatus = true
)

// Result summarizes the result of a EVM code computation.
type Result struct {
	Success   bool // false if the execution ended in a revert, true otherwise
	Output    Data
	GasLeft   Gas
	GasRefund Gas
}

// Data represents the input or output of contract invocations.
type Data []byte

// Gas represents the type used to represent the Gas values.
type Gas int64

// Snapshot is a type used to represent a snapshot of the world state in a
// transaction context.
type Snapshot int

// Log is the type summarizing a log message emitted as a side effect of a
// contract execution.
type Log struct {
	Address Address
	Topics  []Hash
	Data    Data
}

// CallKind is an enum enabling the differentiation of the different types
// of recursive contract calls supported in the EVM.
type CallKind int

const (
	Call CallKind = iota
	DelegateCall
	StaticCall
	CallCode
	Create
	Create2
)

type CallParameters struct {
	Sender      Address // TODO: remove and handle implicit
	Recipient   Address // < not relevant for CREATE and CREATE2 // TODO: remove and handle implicit
	Value       Value   // < ignored by static calls, considered to be 0
	Input       Data
	Gas         Gas
	Salt        Hash // < only relevant for CREATE2 calls
	CodeAddress Address
}

type CallResult struct {
	Output         Data
	GasLeft        Gas
	GasRefund      Gas
	CreatedAddress Address // < only meaningful for CREATE and CREATE2
	Success        bool    // false if the execution ended in a revert, true otherwise
}

// Revision is an enumeration for EVM specification revisions (aka. Hard-Forks).
type Revision int

// The list of revisions supported so far by Tosca.
const (
	R07_Istanbul Revision = iota
	R09_Berlin
	R10_London
	R11_Paris
	R12_Shanghai
	R13_Cancun
	R14_Prague
	R15_Osaka
	numRevisions int = iota
)

// ErrUnsupportedRevision is an error for runs with unsupported Revision
type ErrUnsupportedRevision struct {
	Revision Revision
}

func (e *ErrUnsupportedRevision) Error() string {
	return fmt.Sprintf("unsupported revision %d", e.Revision)
}

// ProfilingInterpreter is an optional extension to the Interpreter interface
// above which may be implemented by interpreters collecting statistical data
// on their executions.
type ProfilingInterpreter interface {
	Interpreter

	// ResetProfile resets the operation statistic collected by the underlying
	// Interpreter implementation. Use this, for instance, at the beginning of
	// a benchmark. It should not be called while running operations on the
	// Interpreter in parallel.
	ResetProfile()

	// DumpProfile prints a snapshot of the profiling data collected since the
	// last reset to stdout. In the future this interface will be changed to
	// return the result instead of printing it.
	// TODO: produce the result as a string
	DumpProfile()
}
