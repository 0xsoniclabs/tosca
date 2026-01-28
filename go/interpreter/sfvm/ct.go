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
	"github.com/0xsoniclabs/tosca/go/ct"
	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/ct/utils"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

func NewConformanceTestingTarget() ct.Evm {

	// Can only fail for invalid configuration. Configuration is hardcoded.
	sanctionedVm, _ := NewInterpreter(Config{})

	return &ctAdapter{
		vm: sanctionedVm,
	}
}

type ctAdapter struct {
	vm *sfvm
}

func (a *ctAdapter) StepN(state *st.State, numSteps int) (*st.State, error) {
	params := utils.ToVmParameters(state)
	if params.Revision > newestSupportedRevision {
		return state, &tosca.ErrUnsupportedRevision{Revision: params.Revision}
	}

	// No need to run anything that is not in a running state.
	if state.Status != st.Running {
		return state, nil
	}

	memory := convertCtMemoryToSfvmMemory(state.Memory)

	// Set up execution context.
	var ctxt = &context{
		pc:           int32(state.Pc),
		params:       params,
		context:      params.Context,
		gas:          params.Gas,
		refund:       tosca.Gas(state.GasRefund),
		stack:        convertCtStackToSfvmStack(state.Stack),
		memory:       memory,
		code:         params.Code,
		analysis:     a.vm.analysis.analyzeJumpDest(params.Code, params.CodeHash),
		returnData:   state.LastCallReturnData.ToBytes(),
		withShaCache: a.vm.config.withShaCache,
	}

	defer func() {
		ReturnStack(ctxt.stack)
	}()

	// Run interpreter.
	status := statusRunning
	for i := 0; status == statusRunning && i < numSteps; i++ {
		status = execute(ctxt, true)
	}

	// Update the resulting state.
	state.Status = convertSfvmStatusToCtStatus(status)

	if status == statusRunning {
		state.Pc = uint16(ctxt.pc)
	}

	state.Gas = ctxt.gas
	state.GasRefund = ctxt.refund
	state.Stack = convertSfvmStackToCtStack(ctxt.stack, state.Stack)
	state.Memory = convertSfvmMemoryToCtMemory(ctxt.memory)
	state.LastCallReturnData = common.NewBytes(ctxt.returnData)
	if status == statusReturned || status == statusReverted {
		state.ReturnData = common.NewBytes(ctxt.returnData)
	}

	return state, nil
}

func convertSfvmStatusToCtStatus(status status) st.StatusCode {
	switch status {
	case statusRunning:
		return st.Running
	case statusReturned, statusStopped:
		return st.Stopped
	case statusReverted:
		return st.Reverted
	case statusSelfDestructed:
		return st.Stopped
	case statusFailed:
		return st.Failed
	}
	return st.Failed
}

func convertCtStackToSfvmStack(stack *st.Stack) *stack {
	result := NewStack()
	for i := stack.Size() - 1; i >= 0; i-- {
		val := stack.Get(i).Uint256()
		result.push(&val)
	}
	return result
}

func convertSfvmStackToCtStack(stack *stack, result *st.Stack) *st.Stack {
	len := stack.len()
	result.Resize(len)
	for i := 0; i < len; i++ {
		result.Set(len-i-1, common.NewU256FromUint256(stack.get(i)))
	}
	return result
}

func convertCtMemoryToSfvmMemory(memory *st.Memory) *Memory {
	data := memory.Read(0, uint64(memory.Size()))
	mem := NewMemory()
	words := tosca.SizeInWords(uint64(len(data)))
	mem.store = make([]byte, words*32)
	copy(mem.store, data)
	mem.currentMemoryCost = tosca.Gas((words*words)/512 + (3 * words))
	return mem
}

func convertSfvmMemoryToCtMemory(memory *Memory) *st.Memory {
	result := st.NewMemory()
	result.Set(memory.store)
	return result
}
