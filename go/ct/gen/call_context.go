// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package gen

import (
	"pgregory.net/rand"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

type CallContextGenerator struct {
}

func NewCallContextGenerator() *CallContextGenerator {
	return &CallContextGenerator{}
}

func (*CallContextGenerator) Generate(rnd *rand.Rand, accountAddress tosca.Address) (st.CallContext, error) {
	callerAddress := common.RandomAddress(rnd)

	return st.CallContext{
		AccountAddress: accountAddress,
		CallerAddress:  callerAddress,
		Value:          common.RandU256(rnd),
	}, nil
}

func (*CallContextGenerator) Clone() *CallContextGenerator {
	return &CallContextGenerator{}
}

func (*CallContextGenerator) Restore(*CallContextGenerator) {
}

func (*CallContextGenerator) String() string {
	return "{}"
}
