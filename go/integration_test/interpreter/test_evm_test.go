// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package interpreter_test

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"go.uber.org/mock/gomock"
)

func TestScenario_CreateAccountAndContractCreateAnEmptySlot(t *testing.T) {
	address := tosca.Address{0x42}
	functions := [2]func(c runContextAdapter){
		func(c runContextAdapter) {
			c.CreateAccount(address)
		},
		func(c runContextAdapter) {
			c.CreateContract(address)
		},
	}

	for _, function := range functions {
		ctrl := gomock.NewController(t)
		stateDB := NewMockStateDB(ctrl)
		// No calls to mock expected, just a clean state.

		contextAdapter := runContextAdapter{
			StateDB: stateDB,
		}
		function(contextAdapter)
	}
}
