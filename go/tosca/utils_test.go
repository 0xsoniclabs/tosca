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

import (
	"math"
	"testing"
)

func TestSizeInWords(t *testing.T) {
	tests := map[string]struct {
		size  uint64
		wants uint64
	}{
		"zero": {
			size:  0,
			wants: 0,
		},
		"one": {
			size:  1,
			wants: 1,
		},
		"31": {
			size:  31,
			wants: 1,
		},
		"32": {
			size:  32,
			wants: 1,
		},
		"33": {
			size:  33,
			wants: 2,
		},
		"64": {
			size:  64,
			wants: 2,
		},
		"65": {
			size:  65,
			wants: 3,
		},
		"maxInt mins 32": {
			size:  uint64(math.MaxUint64) - 32,
			wants: math.MaxUint64 / 32,
		},
		"maxInt mins 31": {
			size:  uint64(math.MaxUint64) - 31,
			wants: math.MaxUint64 / 32,
		},
		"maxInt mins 30": {
			size:  uint64(math.MaxUint64) - 30,
			wants: math.MaxUint64/32 + 1,
		},
		"maxInt mins 1": {
			size:  uint64(math.MaxUint64) - 1,
			wants: math.MaxUint64/32 + 1,
		},
		"maxInt": {
			size:  uint64(math.MaxUint64),
			wants: math.MaxUint64/32 + 1,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			got := SizeInWords(test.size)
			if want := test.wants; want != got {
				t.Errorf("unexpected result, wanted %d, got %d", want, got)
			}
		})
	}
}
