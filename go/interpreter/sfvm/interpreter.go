// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package sfvm

import (
	"fmt"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

// status is enumeration of the execution state of an interpreter run.
type status byte

const (
	statusRunning        status = iota // < all fine, ops are processed
	statusStopped                      // < execution stopped with a STOP
	statusReverted                     // < execution stopped with a REVERT
	statusReturned                     // < execution stopped with a RETURN
	statusSelfDestructed               // < execution stopped with a SELF-DESTRUCT
	statusFailed                       // < execution stopped with a logic error
)

// context is the execution environment of an interpreter run. It contains all
// the necessary state to execute a contract, including input parameters, the
// contract code, and internal execution state such as the program counter,
// stack, and memory. For each contract execution, a new context is created.
type context struct {
	// Inputs
	params   tosca.Parameters
	context  tosca.RunContext
	code     tosca.Code
	analysis jumpDestMap

	// Execution state
	pc     int32
	gas    tosca.Gas
	refund tosca.Gas
	stack  *stack
	memory *Memory

	// Intermediate data
	returnData []byte // < the result of the last nested contract call

	// Configuration flags
	withShaCache bool
}

// useGas reduces the gas level by the given amount. If the gas level drops
// below zero, the caller should stop the execution with an error status. The function
// returns true if sufficient gas was available and execution can continue,
// false otherwise.
func (c *context) useGas(amount tosca.Gas) error {
	if c.gas < 0 || amount < 0 || c.gas < amount {
		return errOutOfGas
	}
	c.gas -= amount
	return nil
}

// isAtLeast returns true if the interpreter is is running at least at the given
// revision or newer, false otherwise.
func (c *context) isAtLeast(revision tosca.Revision) bool {
	return c.params.Revision >= revision
}

func run(
	analysis analysis,
	config config,
	params tosca.Parameters,
) (tosca.Result, error) {
	// Don't bother with the execution if there's no code.
	if len(params.Code) == 0 {
		return tosca.Result{
			Output:  nil,
			GasLeft: params.Gas,
			Success: true,
		}, nil
	}

	// Set up execution context.
	var ctxt = context{
		params:       params,
		context:      params.Context,
		gas:          params.Gas,
		stack:        NewStack(),
		memory:       NewMemory(),
		code:         params.Code,
		analysis:     analysis.analyzeJumpDest(params.Code, params.CodeHash),
		withShaCache: config.withShaCache,
	}
	defer ReturnStack(ctxt.stack)

	status := execute(&ctxt, false)
	return generateResult(status, &ctxt)
}

func generateResult(status status, ctxt *context) (tosca.Result, error) {
	// Handle return status
	switch status {
	case statusStopped, statusSelfDestructed:
		return tosca.Result{
			Success:   true,
			GasLeft:   ctxt.gas,
			GasRefund: ctxt.refund,
		}, nil
	case statusReturned:
		return tosca.Result{
			Success:   true,
			Output:    ctxt.returnData,
			GasLeft:   ctxt.gas,
			GasRefund: ctxt.refund,
		}, nil
	case statusReverted:
		return tosca.Result{
			Success: false,
			Output:  ctxt.returnData,
			GasLeft: ctxt.gas,
		}, nil
	case statusFailed:
		return tosca.Result{
			Success: false,
		}, nil
	default:
		return tosca.Result{}, fmt.Errorf("unexpected error in interpreter, unknown status: %v", status)
	}
}

// --- Execution ---

// execute runs the contract code in the given context. If oneStepOnly is true,
// only the instruction pointed to by the program counter will be executed.
// If the contract execution yields any execution violation (i.e. out of gas,
// stack underflow, etc), the function returns statusFailed
func execute(c *context, oneStepOnly bool) status {
	status, error := steps(c, oneStepOnly)
	if error != nil {
		return statusFailed
	}
	return status
}

// steps executes the contract code in the given context,
// If oneStepOnly is true, only the instruction pointed to by the program
// counter will be executed.
// steps returns the status of the execution and an error if the contract
// execution yields any execution violation (i.e. out of gas, stack underflow, etc).
func steps(c *context, oneStepOnly bool) (status, error) {
	staticGasPrices := getStaticGasPrices(c.params.Revision)

	status := statusRunning
	for status == statusRunning {
		if int(c.pc) >= len(c.code) {
			return statusStopped, nil
		}

		op := vm.OpCode(c.code[c.pc])

		// Check stack boundary for every instruction
		if err := checkStackLimits(c.stack.len(), op); err != nil {
			return status, err
		}

		// Consume static gas price for instruction before execution
		if err := c.useGas(staticGasPrices.get(op)); err != nil {
			return status, err
		}

		var err error

		// Execute instruction
		switch op {
		case vm.POP:
			opPop(c)
		case vm.PUSH0:
			err = opPush0(c)
		case vm.PUSH1:
			opPush1(c)
		case vm.PUSH2:
			opPush2(c)
		case vm.PUSH3:
			opPush3(c)
		case vm.PUSH4:
			opPush4(c)
		case vm.PUSH5:
			opPush(c, 5)
		case vm.PUSH31:
			opPush(c, 31)
		case vm.PUSH32:
			opPush32(c)
		case vm.JUMP:
			err = opJump(c)
		case vm.JUMPDEST:
			// nothing
		case vm.SWAP1:
			opSwap(c, 1)
		case vm.SWAP2:
			opSwap(c, 2)
		case vm.DUP3:
			opDup(c, 3)
		case vm.AND:
			opAnd(c)
		case vm.SWAP3:
			opSwap(c, 3)
		case vm.JUMPI:
			err = opJumpi(c)
		case vm.GT:
			opGt(c)
		case vm.DUP4:
			opDup(c, 4)
		case vm.DUP2:
			opDup(c, 2)
		case vm.ISZERO:
			opIszero(c)
		case vm.ADD:
			opAdd(c)
		case vm.OR:
			opOr(c)
		case vm.XOR:
			opXor(c)
		case vm.NOT:
			opNot(c)
		case vm.SUB:
			opSub(c)
		case vm.MUL:
			opMul(c)
		case vm.MULMOD:
			opMulMod(c)
		case vm.DIV:
			opDiv(c)
		case vm.SDIV:
			opSDiv(c)
		case vm.MOD:
			opMod(c)
		case vm.SMOD:
			opSMod(c)
		case vm.ADDMOD:
			opAddMod(c)
		case vm.EXP:
			err = opExp(c)
		case vm.DUP5:
			opDup(c, 5)
		case vm.DUP1:
			opDup(c, 1)
		case vm.EQ:
			opEq(c)
		case vm.PC:
			opPc(c)
		case vm.CALLER:
			opCaller(c)
		case vm.CALLDATALOAD:
			opCallDataload(c)
		case vm.CALLDATASIZE:
			opCallDatasize(c)
		case vm.CALLDATACOPY:
			err = genericDataCopy(c, c.params.Input)
		case vm.MLOAD:
			err = opMload(c)
		case vm.MSTORE:
			err = opMstore(c)
		case vm.MSTORE8:
			err = opMstore8(c)
		case vm.MSIZE:
			opMsize(c)
		case vm.MCOPY:
			err = opMcopy(c)
		case vm.LT:
			opLt(c)
		case vm.SLT:
			opSlt(c)
		case vm.SGT:
			opSgt(c)
		case vm.SHR:
			opShr(c)
		case vm.SHL:
			opShl(c)
		case vm.SAR:
			opSar(c)
		case vm.CLZ:
			err = opClz(c)
		case vm.SIGNEXTEND:
			opSignExtend(c)
		case vm.BYTE:
			opByte(c)
		case vm.SHA3:
			err = opSha3(c)
		case vm.CALLVALUE:
			opCallvalue(c)
		case vm.PUSH6:
			opPush(c, 6)
		case vm.PUSH7:
			opPush(c, 7)
		case vm.PUSH8:
			opPush(c, 8)
		case vm.PUSH9:
			opPush(c, 9)
		case vm.PUSH10:
			opPush(c, 10)
		case vm.PUSH11:
			opPush(c, 11)
		case vm.PUSH12:
			opPush(c, 12)
		case vm.PUSH13:
			opPush(c, 13)
		case vm.PUSH14:
			opPush(c, 14)
		case vm.PUSH15:
			opPush(c, 15)
		case vm.PUSH16:
			opPush(c, 16)
		case vm.PUSH17:
			opPush(c, 17)
		case vm.PUSH18:
			opPush(c, 18)
		case vm.PUSH19:
			opPush(c, 19)
		case vm.PUSH20:
			opPush(c, 20)
		case vm.PUSH21:
			opPush(c, 21)
		case vm.PUSH22:
			opPush(c, 22)
		case vm.PUSH23:
			opPush(c, 23)
		case vm.PUSH24:
			opPush(c, 24)
		case vm.PUSH25:
			opPush(c, 25)
		case vm.PUSH26:
			opPush(c, 26)
		case vm.PUSH27:
			opPush(c, 27)
		case vm.PUSH28:
			opPush(c, 28)
		case vm.PUSH29:
			opPush(c, 29)
		case vm.PUSH30:
			opPush(c, 30)
		case vm.SWAP4:
			opSwap(c, 4)
		case vm.SWAP5:
			opSwap(c, 5)
		case vm.SWAP6:
			opSwap(c, 6)
		case vm.SWAP7:
			opSwap(c, 7)
		case vm.SWAP8:
			opSwap(c, 8)
		case vm.SWAP9:
			opSwap(c, 9)
		case vm.SWAP10:
			opSwap(c, 10)
		case vm.SWAP11:
			opSwap(c, 11)
		case vm.SWAP12:
			opSwap(c, 12)
		case vm.SWAP13:
			opSwap(c, 13)
		case vm.SWAP14:
			opSwap(c, 14)
		case vm.SWAP15:
			opSwap(c, 15)
		case vm.SWAP16:
			opSwap(c, 16)
		case vm.DUP6:
			opDup(c, 6)
		case vm.DUP7:
			opDup(c, 7)
		case vm.DUP8:
			opDup(c, 8)
		case vm.DUP9:
			opDup(c, 9)
		case vm.DUP10:
			opDup(c, 10)
		case vm.DUP11:
			opDup(c, 11)
		case vm.DUP12:
			opDup(c, 12)
		case vm.DUP13:
			opDup(c, 13)
		case vm.DUP14:
			opDup(c, 14)
		case vm.DUP15:
			opDup(c, 15)
		case vm.DUP16:
			opDup(c, 16)
		case vm.RETURN:
			err = opEndWithResult(c)
			status = statusReturned
		case vm.REVERT:
			status = statusReverted
			err = opEndWithResult(c)
		case vm.SLOAD:
			err = opSload(c)
		case vm.SSTORE:
			err = opSstore(c)
		case vm.TLOAD:
			err = opTload(c)
		case vm.TSTORE:
			err = opTstore(c)
		case vm.CODESIZE:
			opCodeSize(c)
		case vm.CODECOPY:
			err = genericDataCopy(c, c.params.Code)
		case vm.EXTCODESIZE:
			err = opExtcodesize(c)
		case vm.EXTCODEHASH:
			err = opExtcodehash(c)
		case vm.EXTCODECOPY:
			err = opExtCodeCopy(c)
		case vm.BALANCE:
			err = opBalance(c)
		case vm.SELFBALANCE:
			opSelfbalance(c)
		case vm.BASEFEE:
			err = opBaseFee(c)
		case vm.BLOBHASH:
			err = opBlobHash(c)
		case vm.BLOBBASEFEE:
			err = opBlobBaseFee(c)
		case vm.SELFDESTRUCT:
			status, err = opSelfdestruct(c)
		case vm.CHAINID:
			opChainId(c)
		case vm.GAS:
			opGas(c)
		case vm.PREVRANDAO:
			opPrevRandao(c)
		case vm.TIMESTAMP:
			opTimestamp(c)
		case vm.NUMBER:
			opNumber(c)
		case vm.GASLIMIT:
			opGasLimit(c)
		case vm.GASPRICE:
			opGasPrice(c)
		case vm.CALL:
			err = opCall(c)
		case vm.CALLCODE:
			err = opCallCode(c)
		case vm.STATICCALL:
			err = opStaticCall(c)
		case vm.DELEGATECALL:
			err = opDelegateCall(c)
		case vm.RETURNDATASIZE:
			opReturnDataSize(c)
		case vm.RETURNDATACOPY:
			err = opReturnDataCopy(c)
		case vm.BLOCKHASH:
			opBlockhash(c)
		case vm.COINBASE:
			opCoinbase(c)
		case vm.ORIGIN:
			opOrigin(c)
		case vm.ADDRESS:
			opAddress(c)
		case vm.STOP:
			status = opStop()
		case vm.CREATE:
			err = genericCreate(c, tosca.Create)
		case vm.CREATE2:
			err = genericCreate(c, tosca.Create2)
		case vm.LOG0:
			err = opLog(c, 0)
		case vm.LOG1:
			err = opLog(c, 1)
		case vm.LOG2:
			err = opLog(c, 2)
		case vm.LOG3:
			err = opLog(c, 3)
		case vm.LOG4:
			err = opLog(c, 4)
		default:
			err = errInvalidOpCode
		}

		if err != nil {
			return status, err
		}

		c.pc++

		if oneStepOnly {
			return status, nil
		}
	}
	return status, nil
}

// checkStackLimits checks that the opCode will not make an out of bounds access
// with the current stack size.
func checkStackLimits(stackLen int, op vm.OpCode) error {
	limits := _precomputedStackLimits.get(op)
	if stackLen < limits.min {
		return errStackUnderflow
	}
	if stackLen > limits.max {
		return errStackOverflow
	}
	return nil
}

// stackLimits defines the stack usage of a single OpCode.
type stackLimits struct {
	min int // The minimum stack size required by an OpCode.
	max int // The maximum stack size allowed before running an OpCode.
}

var _precomputedStackLimits = newOpCodePropertyMap(func(op vm.OpCode) stackLimits {
	usage := computeStackUsage(op)
	return stackLimits{
		min: -usage.from,
		max: maxStackSize - usage.to,
	}
})
