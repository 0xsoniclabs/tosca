// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package integration_test

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/crypto"
)

func TestKeccak256Hash_EmptyInputReturnsKnownHash(t *testing.T) {
	data := []byte{}
	want := tosca.Hash(crypto.Keccak256(data))
	got := Keccak256Hash(data)
	if got != want {
		t.Errorf("Keccak256Hash did not return the empty hash")
	}

}
