// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package common

import (
	"crypto/rand"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
)

func TestParseDelegationDesignator(t *testing.T) {

	address := tosca.Address{}
	_, _ = rand.Read(address[:])

	tests := map[string]struct {
		code           Bytes
		expectedParsed bool
		expectedResult tosca.Address
	}{
		"empty": {},
		"too short": {
			code: NewBytes([]byte{0xef, 0x01, 0x00, 0xab}),
		},
		"delegation designator": {
			code:           NewBytes(append([]byte{0xef, 0x01, 0x00}, address[:]...)),
			expectedParsed: true,
			expectedResult: address,
		},
		"too long": {
			code: NewBytes(append(append([]byte{0xef, 0x01, 0x00}, address[:]...), 0xab)),
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {

			result, parsed := ParseDelegationDesignator(test.code)
			if parsed != test.expectedParsed {
				t.Errorf("want: %v, got: %v", test.expectedParsed, parsed)
			}

			if test.expectedParsed && result != test.expectedResult {
				t.Errorf("want: %v, got: %v", result, test.expectedParsed)
			}
		})
	}
}

func TestDelegationDesignator_CanBeWrittenAndParsed(t *testing.T) {
	address := tosca.Address{}
	_, _ = rand.Read(address[:])
	designator := NewDelegationDesignator(address)

	result, parsed := ParseDelegationDesignator(designator)
	if !parsed {
		t.Errorf("could not parse delegation designator")
	}

	if address != result {
		t.Errorf("want: %v, got: %v", &address, parsed)
	}
}
