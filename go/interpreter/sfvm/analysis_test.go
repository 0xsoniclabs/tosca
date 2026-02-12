// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package sfvm

import (
	"bytes"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	"github.com/stretchr/testify/require"
)

func TestAnalysisCache_PanicsOnNegativeSize(t *testing.T) {
	require.Panics(t, func() {
		newAnalysis(-1, 1)
	})
}

func TestAnalysis_NewAnalysisIsNonEmpty(t *testing.T) {
	a := newJumpDestMap(10)
	if a.codeSize == 0 {
		t.Error("expected newAnalysis to return a non-empty Analysis")
	}
	if len(a.bitmap) == 0 {
		t.Error("expected newAnalysis to return a non-empty data slice")
	}
}

func TestAnalysis_NewJumpDestMapIsSufficientlyLarge(t *testing.T) {
	for size := uint64(1); size < 1000000; size *= 10 {
		a := newJumpDestMap(size)
		idx, _ := idxToAnalysisIdxAndMask(size - 1)

		if len(a.bitmap) <= int(idx) {
			t.Fatal("access index is larger than jump dest bitmap")
		}
	}
}

func TestAnalysis_BitmapMaskAndIndexAreContinuous(t *testing.T) {
	prevMask := uint64(1)
	prevIdx := uint64(0)

	skipShift := true
	for inputIdx := range 1000 {
		idx, mask := idxToAnalysisIdxAndMask(uint64(inputIdx))
		if idx != prevIdx {
			if prevIdx+1 != idx {
				t.Fatalf("Index is not continuous expected %v, got %v", prevIdx+1, idx)
			}
			prevMask = 1
			skipShift = true
			prevIdx = idx
		}

		expectedMask := prevMask << 1
		// It is not possible set a value that is 1 after the left shift,
		// therefore the skip has to be undone for every first uint64 access.
		if skipShift {
			expectedMask = expectedMask >> 1
			skipShift = false
		}
		if expectedMask != mask {
			t.Fatalf("Mask is not continuous expected %v, got %v", expectedMask, mask)
		}
		prevMask = mask
	}
}

func TestAnalysis_MarkJumpDestAndIsJumpDest(t *testing.T) {
	size := 10
	a := newJumpDestMap(uint64(size))
	a.markJumpDest(2)
	a.markJumpDest(18)
	// Check that the jump destination is marked correctly over boundaries
	for i := 0; i < 2*size; i++ {
		if i == 2 && !a.isJumpDest(uint64(i)) {
			t.Errorf("expected index %d to be marked as jump destination", i)
		}
		if i != 2 && a.isJumpDest(uint64(i)) {
			t.Errorf("expected index %d to not be marked as jump destination", i)
		}
	}
}

func TestAnalysis_MarkJumpDestDoesNotCrashWithWronglySetUpJumpDestMap(t *testing.T) {
	size := 200
	analysis := newJumpDestMap(uint64(size))
	analysis.bitmap = analysis.bitmap[:3] // Incorrectly resize the bitmap to be smaller
	analysis.markJumpDest(2)
	analysis.markJumpDest(199) // No out of bounds crash

	// Index 2 should still be marked correctly
	if !analysis.isJumpDest(2) {
		t.Errorf("expected index 2 to be marked as jump destination")
	}

	// Index 199 is out of bounds and should therefore not be marked
	if analysis.isJumpDest(199) {
		t.Errorf("expected index 199 to not be marked as jump destination")
	}
}

func TestAnalysis_MarksJumpDestAtCorrectIndex(t *testing.T) {
	code := tosca.Code{byte(vm.JUMPDEST), byte(vm.PUSH1), byte(vm.JUMPDEST), byte(vm.JUMPDEST)}
	analysis := findJumpDestinations(code)
	if !analysis.isJumpDest(0) {
		t.Errorf("expected index 0 to be jump destination")
	}
	if analysis.isJumpDest(1) {
		t.Errorf("expected index 1 to not be jump destination")
	}
	if analysis.isJumpDest(2) {
		t.Errorf("expected index 2 to not be jump destination")
	}
	if !analysis.isJumpDest(3) {
		t.Errorf("expected index 3 to be jump destination")
	}
}

func TestAnalysis_PushDataIsSkipped(t *testing.T) {
	code := tosca.Code{
		byte(vm.PUSH9), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST),
		byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST),
		byte(vm.JUMPDEST),
		byte(vm.PUSH2), byte(vm.JUMPDEST), byte(vm.JUMPDEST),
		byte(vm.JUMPDEST),
	}
	analysis := findJumpDestinations(code)
	for i := range code {
		if analysis.isJumpDest(uint64(i)) && (i != 10 && i != 14) {
			t.Errorf("expected index %d to be jump destination", i)
		}
		if !analysis.isJumpDest(uint64(i)) && (i == 10 || i == 14) {
			t.Errorf("expected index %d to not be jump destination", i)
		}
	}
}

func TestAnalysis_InputsAreCachedUsingCodeHashAsKey(t *testing.T) {
	analysis := newAnalysis(1<<2, 1)

	code := []byte{byte(vm.STOP)}
	hash := tosca.Hash{byte(1)}

	want := analysis.analyzeJumpDest(code, &hash)
	got := analysis.analyzeJumpDest(code, &hash)
	if &want.bitmap[0] != &got.bitmap[0] {
		t.Fatal("cached conversion result not returned")
	}
}

func TestAnalysis_CodesBiggerThanMaxCachedLengthAreNotCached(t *testing.T) {
	maxCachedCodeLength := 2

	tests := map[string]struct {
		code   tosca.Code
		cached bool
	}{
		"maxCachedCodeLength-1": {
			code:   bytes.Repeat([]byte{0x42}, maxCachedCodeLength-1),
			cached: true,
		},
		"maxCachedCodeLength": {
			code:   bytes.Repeat([]byte{0x42}, maxCachedCodeLength),
			cached: true,
		},
		"maxCachedCodeLength+1": {
			code:   bytes.Repeat([]byte{0x42}, maxCachedCodeLength+1),
			cached: false,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			analysis := newAnalysis(1<<2, maxCachedCodeLength)

			hash := tosca.Hash{byte(1)}
			analysis0 := analysis.analyzeJumpDest(test.code, &hash)
			analysis1 := analysis.analyzeJumpDest(test.code, &hash)

			want := &analysis0.bitmap[0]
			got := &analysis1.bitmap[0]

			if test.cached && want != got {
				t.Fatal("expected conversion result to be cached")
			}
			if !test.cached && want == got {
				t.Fatal("expected conversion result to not be cached")
			}
		})
	}
}

func TestAnalysis_NewAnalysisEnsuresMaxCachedCodeSizeIsInBounds(t *testing.T) {
	cacheSize := 128
	tests := map[string]int{
		"negative maxCachedCodeSize": -1,
		"zero maxCachedCodeSize":     0,
		"positive maxCachedCodeSize": 1,
		"cacheSize":                  cacheSize,
		"lager than cacheSize":       cacheSize + 1,
		"huge":                       1 << 30,
	}

	for name, maxCachedCodeSize := range tests {
		t.Run(name, func(t *testing.T) {
			analysis := newAnalysis(cacheSize, maxCachedCodeSize)

			if analysis.maxCachedCodeSize < 1 {
				t.Fatalf("maxCachedCodeSize is smaller than 1: %d", analysis.maxCachedCodeSize)
			}
			if analysis.maxCachedCodeSize > cacheSize {
				t.Fatalf("maxCachedCodeSize is larger than cache size: %d", analysis.maxCachedCodeSize)
			}
		})
	}
}

func TestAnalysis_IsJumpDestCanHandleUninitializedMap(t *testing.T) {
	var jumpDestMap *jumpDestMap
	require.False(t, jumpDestMap.isJumpDest(uint64(0)), "expected isJumpDest to return false for uninitialized map")
}
