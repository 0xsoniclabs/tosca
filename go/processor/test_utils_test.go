// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package test_utils

import "testing"

func TestNewAddressHasRightEndian(t *testing.T) {
	addr := NewAddress(1)
	if addr[19] != 1 {
		t.Errorf("Expected 1, got %d", addr[0])
	}
	for b := range addr[0:19] {
		if addr[b] != 0 {
			t.Errorf("Expected 0, got %d", addr[b])
		}
	}
}
