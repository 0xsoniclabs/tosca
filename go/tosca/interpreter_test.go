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

import "testing"

func TestSplitStateGasCharge(t *testing.T) {
	tests := map[string]struct {
		reservoir     Gas
		charged       Gas
		fromReservoir Gas
		fromGas       Gas
	}{
		"nothing charged":            {reservoir: 100, charged: 0, fromReservoir: 0, fromGas: 0},
		"covered by the reservoir":   {reservoir: 100, charged: 40, fromReservoir: 40, fromGas: 0},
		"exhausts the reservoir":     {reservoir: 100, charged: 100, fromReservoir: 100, fromGas: 0},
		"exceeds the reservoir by 1": {reservoir: 100, charged: 101, fromReservoir: 100, fromGas: 1},
		"without a reservoir":        {reservoir: 0, charged: 40, fromReservoir: 0, fromGas: 40},
		// A net state-gas refund can leave the charge negative; there is no
		// regular gas to be repaid in that case.
		"net refund": {reservoir: 100, charged: -40, fromReservoir: -40, fromGas: 0},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			fromReservoir, fromGas := SplitStateGasCharge(test.reservoir, test.charged)
			if fromReservoir != test.fromReservoir || fromGas != test.fromGas {
				t.Errorf("unexpected split, wanted (%d,%d), got (%d,%d)",
					test.fromReservoir, test.fromGas, fromReservoir, fromGas)
			}
			if got, want := fromReservoir+fromGas, test.charged; got != want {
				t.Errorf("split does not add up, wanted %d, got %d", want, got)
			}
			if fromReservoir > test.reservoir {
				t.Errorf("split takes %d from a reservoir of %d", fromReservoir, test.reservoir)
			}
		})
	}
}

// TestGas_ZeroStateGasChargeLeavesTheReservoirUntouched pins the property the
// interface relies on for interpreters that do not implement EIP-8037: their
// zero-valued report must not consume any of the reservoir.
func TestGas_ZeroStateGasChargeLeavesTheReservoirUntouched(t *testing.T) {
	reservoir := Gas(12345)
	fromReservoir, fromGas := SplitStateGasCharge(reservoir, Result{}.StateGasCharged)
	if fromReservoir != 0 || fromGas != 0 {
		t.Errorf("a default result consumes gas, got (%d,%d)", fromReservoir, fromGas)
	}
}
