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
	"github.com/0xsoniclabs/tosca/go/tosca"
	"golang.org/x/crypto/sha3"
)

func Keccak256Hash(data []byte) tosca.Hash {
	hasher := sha3.NewLegacyKeccak256()
	hasher.Write(data)
	var hash tosca.Hash
	hasher.Sum(hash[0:0])
	return hash
}
