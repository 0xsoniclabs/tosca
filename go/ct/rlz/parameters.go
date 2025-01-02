// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package rlz

import (
	"math"

	"github.com/0xsoniclabs/Tosca/go/ct/common"
	"github.com/0xsoniclabs/Tosca/go/ct/st"
)

type Parameter interface {
	// Samples returns a list of test values for an operation parameter.
	// For efficiency reasons, the resulting slice may be shared among
	// multiple calls. Thus, neither the slice itself nor its elements must
	// be modified by users of this function.
	Samples() []common.U256
}

type NumericParameter struct{}

var numericParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.NewU256(1 << 8),
	common.NewU256(1 << 16),
	common.NewU256(1 << 32),
	common.NewU256(1 << 48),
	common.NewU256(1).Shl(common.NewU256(64)),
	common.NewU256(1).Shl(common.NewU256(128)),
	common.NewU256(1).Shl(common.NewU256(192)),
	common.NewU256(1).Shl(common.NewU256(255)),
	common.NewU256(0).Not(),
	common.NewU256(1, 1),
}

func (NumericParameter) Samples() []common.U256 {
	return numericParameterSamples
}

type JumpTargetParameter struct{}

var jumpTargetParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.NewU256(1 << 8),
	common.NewU256(math.MaxInt32 + 1),
	common.NewU256(1, 1),
}

func (JumpTargetParameter) Samples() []common.U256 {
	return jumpTargetParameterSamples
}

type StorageAccessKeyParameter = NumericParameter

// MemoryOffsetParameter is a parameter for offsets used when accessing memory.
type MemoryOffsetParameter struct{}

var memoryOffsetParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.NewU256(32),
	common.NewU256(st.MaxMemoryExpansionSize),
	common.NewU256(st.MaxMemoryExpansionSize + 1),
	common.NewU256(1, 0),
}

func (MemoryOffsetParameter) Samples() []common.U256 {
	return memoryOffsetParameterSamples
}

// DataOffsetParameter is a parameter for offsets used when accessing other
// buffers but Memory (input, code, return).
// Because these buffers are not expanding memory, much larger offsets are valid.
type DataOffsetParameter struct{}

var dataOffsetParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.NewU256(32),
	common.NewU256(math.MaxUint64),
	common.MaxU256(),
}

func (DataOffsetParameter) Samples() []common.U256 {
	return dataOffsetParameterSamples
}

// SizeParameter is a parameter for sizes used when accessing buffers,
// Ops involving both memory and a second buffer use one single size parameter.
type SizeParameter struct{}

var sizeParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.NewU256(32),
	common.NewU256(1, 0),

	// Samples stressing the max init code size introduced with Shanghai
	common.NewU256(2*24576 - 1),
	common.NewU256(2 * 24576),
	common.NewU256(2*24576 + 1),

	common.NewU256(st.MaxMemoryExpansionSize),
	common.NewU256(st.MaxMemoryExpansionSize + 1),
}

func (SizeParameter) Samples() []common.U256 {
	return sizeParameterSamples
}

type TopicParameter struct{}

var topicParameterSamples = []common.U256{
	// Two samples to ensure topic order is correct. Adding more samples
	// here will create significant more test cases for LOG instructions.
	common.NewU256(101),
	common.NewU256(102),
}

func (TopicParameter) Samples() []common.U256 {
	return topicParameterSamples
}

type AddressParameter struct{}

var addressParameterSamples = []common.U256{
	// Adding more samples here will create significantly more test cases for EXTCODECOPY.
	// TODO: evaluate code coverage
	common.NewU256(0),
	//NewU256(1),
	//NewU256(1).Shl(NewU256(20*8 - 1)), // < first bit of 20-byte address set
	//NewU256(3).Shl(NewU256(20*8 - 1)), // < first bit beyond 20-byte address set as well (should be the same address as above)
	common.NewU256(0).Not(),
}

func (AddressParameter) Samples() []common.U256 {
	return addressParameterSamples
}

type GasParameter struct{}

var gasParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.NewU256(math.MaxInt64),
	common.NewU256(math.MaxInt64 + 1),
}

func (GasParameter) Samples() []common.U256 {
	return gasParameterSamples
}

type ValueParameter struct{}

var valueParameterSamples = []common.U256{
	common.NewU256(0),
	common.NewU256(1),
	common.MaxU256(),
}

func (ValueParameter) Samples() []common.U256 {
	return valueParameterSamples
}
