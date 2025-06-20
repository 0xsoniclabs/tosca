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
	"bytes"

	"github.com/0xsoniclabs/tosca/go/tosca"
)

// ParseDelegationDesignator returns the delegate from a code
// segment containing a delegation designator, if any.
// If the code segment does not contain a delegation designator,
// second returned value is false, and the address shall be ignored.
// see: https://eips.ethereum.org/EIPS/eip-7702
func ParseDelegationDesignator(code Bytes) (tosca.Address, bool) {
	raw := code.ToBytes()

	if len(raw) != 23 {
		return tosca.Address{}, false
	}

	if bytes.HasPrefix(raw, []byte{0xef, 0x01, 0x00}) {
		var res tosca.Address
		copy(res[:], raw[3:23])
		return res, true
	}
	return tosca.Address{}, false
}

// NewDelegationDesignator creates a new delegation designator for the given address.
// see: https://eips.ethereum.org/EIPS/eip-7702
func NewDelegationDesignator(address tosca.Address) Bytes {
	return NewBytes(append([]byte{0xef, 0x01, 0x00}, address[:]...))
}
