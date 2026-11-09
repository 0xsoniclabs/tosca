// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth

import (
	"fmt"
	"testing"

	"github.com/0xsoniclabs/tosca/go/geth_adapter"
	"github.com/0xsoniclabs/tosca/go/tosca"
	geth_common "github.com/ethereum/go-ethereum/common"
	geth_vm "github.com/ethereum/go-ethereum/core/vm"
	"github.com/holiman/uint256"
	"go.uber.org/mock/gomock"
)

func TestCallInterceptor_ForwardsExecutionGasAndReturnsStateGasUnchanged(t *testing.T) {
	const executionGas, stateGas, gasLeft = 1000, 300, 400

	me := geth_common.Address{1}
	target := geth_common.Address{2}
	value := uint256.NewInt(0)

	intercepts := map[tosca.CallKind]func(*callInterceptor, geth_vm.GasBudget) geth_vm.GasBudget{
		tosca.Call: func(i *callInterceptor, gas geth_vm.GasBudget) geth_vm.GasBudget {
			_, left, _ := i.Call(nil, me, target, nil, gas, value)
			return left
		},
		tosca.CallCode: func(i *callInterceptor, gas geth_vm.GasBudget) geth_vm.GasBudget {
			_, left, _ := i.CallCode(nil, me, target, nil, gas, value)
			return left
		},
		tosca.DelegateCall: func(i *callInterceptor, gas geth_vm.GasBudget) geth_vm.GasBudget {
			_, left, _ := i.DelegateCall(nil, me, target, nil, gas)
			return left
		},
		tosca.StaticCall: func(i *callInterceptor, gas geth_vm.GasBudget) geth_vm.GasBudget {
			_, left, _ := i.StaticCall(nil, me, target, nil, gas)
			return left
		},
		tosca.Create: func(i *callInterceptor, gas geth_vm.GasBudget) geth_vm.GasBudget {
			_, _, left, _ := i.Create(nil, me, nil, gas, value)
			return left
		},
		tosca.Create2: func(i *callInterceptor, gas geth_vm.GasBudget) geth_vm.GasBudget {
			_, _, left, _ := i.Create2(nil, me, nil, gas, value, uint256.NewInt(0))
			return left
		},
	}

	for kind, intercept := range intercepts {
		for _, success := range []bool{true, false} {
			t.Run(fmt.Sprintf("%v/success=%t", kind, success), func(t *testing.T) {
				ctrl := gomock.NewController(t)
				context := tosca.NewMockRunContext(ctrl)
				context.EXPECT().GetBalance(gomock.Any()).AnyTimes()
				context.EXPECT().Call(kind, gomock.Any()).DoAndReturn(
					func(_ tosca.CallKind, params tosca.CallParameters) (tosca.CallResult, error) {
						if params.Gas != executionGas {
							t.Errorf("unexpected gas forwarded, wanted %d, got %d", executionGas, params.Gas)
						}
						return tosca.CallResult{GasLeft: gasLeft, Success: success}, nil
					})

				interceptor := &callInterceptor{
					parameters: tosca.Parameters{Context: context},
					stateDb:    geth_adapter.NewStateDB(context),
				}
				got := intercept(interceptor, geth_vm.NewGasBudget(executionGas, stateGas))
				if want := geth_vm.NewGasBudget(gasLeft, stateGas); got != want {
					t.Errorf("unexpected gas budget, wanted %v, got %v", want, got)
				}
			})
		}
	}
}
