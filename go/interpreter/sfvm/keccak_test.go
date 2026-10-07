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
	"encoding/hex"
	"fmt"
	"math/rand"
	"sync"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/stretchr/testify/require"
)

func TestKeccak256_MatchesKnownAnswers(t *testing.T) {
	// Published Keccak-256 digests (not SHA3-256, which pads differently).
	tests := map[string]struct {
		input []byte
		want  string
	}{
		"empty input":   {input: []byte{}, want: "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"},
		"abc":           {input: []byte("abc"), want: "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"},
		"32 zero bytes": {input: make([]byte, 32), want: "290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563"},
		"quick brown fox": {
			input: []byte("The quick brown fox jumps over the lazy dog"),
			want:  "4d741b6f1eb29cb2a9b9911c82f56fa8d73b04959d3d9d222895df6c0b28aa15",
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			require := require.New(t)
			want, err := hex.DecodeString(test.want)
			require.NoError(err)

			require.Equal(tosca.Hash(want), Keccak256(test.input))
			require.Equal(tosca.Hash(want), keccak256_Go(test.input))
		})
	}
}

func TestKeccak256For32byte_MatchesKnownAnswer(t *testing.T) {
	require := require.New(t)
	want, err := hex.DecodeString("290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563")
	require.NoError(err)

	require.Equal(tosca.Hash(want), Keccak256For32byte([32]byte{}))
}

func TestKeccakGo_IsSafeForConcurrentUse(t *testing.T) {
	require := require.New(t)
	const (
		goroutines = 8
		iterations = 1_000
	)

	errs := make(chan error, goroutines)
	var wg sync.WaitGroup
	for goroutine := range goroutines {
		wg.Go(func() {
			data := make([]byte, 200)
			for i := range iterations {
				data[0], data[1], data[199] = byte(goroutine), byte(i), byte(i>>8)
				if want, got := keccak256_C(data), keccak256_Go(data); want != got {
					errs <- fmt.Errorf("goroutine %d got %x for input %d, want %x", goroutine, got, i, want)
					return
				}
			}
		})
	}
	wg.Wait()
	close(errs)

	for err := range errs {
		require.NoError(err)
	}
}

func TestKeccakC_ProducesSameHashAsGo(t *testing.T) {
	tests := [][]byte{
		nil,
		{},
		{1, 2, 3},
		{1, 2, 3, 4, 5, 6, 7, 8, 9, 10},
		make([]byte, 128),
		make([]byte, 1024),
	}
	for _, test := range tests {
		want := keccak256_Go(test)
		got := keccak256_C(test)
		if want != got {
			t.Errorf("unexpected hash for %v, wanted %v, got %v", test, want, got)
		}
	}
}

func TestKeccakC_ProducesSameHashAsGoForEveryLengthUpToTwoBlocks(t *testing.T) {
	// Keccak-256 absorbs 136-byte blocks; lengths up to 300 cover the padding
	// at both block boundaries with non-zero data.
	require := require.New(t)
	r := rand.New(rand.NewSource(7))

	for length := range 301 {
		data := make([]byte, length)
		r.Read(data)

		require.Equal(keccak256_Go(data), keccak256_C(data))
	}
}

func TestKeccakC_32ByteSpecializationProducesSameHashAsGenericVersion(t *testing.T) {
	tests := [][32]byte{
		{},
		{1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2},
	}

	// Test each individual bit.
	for i := range 32 * 8 {
		data := [32]byte{}
		data[i/8] = 1 << (i % 8)
		tests = append(tests, data)
	}

	// Add some random inputs as well.
	r := rand.New(rand.NewSource(99))
	for range 10 {
		data := [32]byte{}
		r.Read(data[:])
		tests = append(tests, data)
	}

	t.Run("keccak256_C_32byte", func(t *testing.T) {
		t.Parallel()
		for _, test := range tests {
			want := keccak256_Go(test[:])
			got := keccak256_C_32byte(test)
			if want != got {
				t.Errorf("unexpected hash for %v, wanted %v, got %v", test, want, got)
			}
		}
	})

	t.Run("Keccak256For32byte", func(t *testing.T) {
		t.Parallel()
		for _, test := range tests {
			want := keccak256_Go(test[:])
			got := Keccak256For32byte(test)
			if want != got {
				t.Errorf("unexpected hash for %v, wanted %v, got %v", test, want, got)
			}
		}
	})
}

func benchmark(b *testing.B, hasher func([]byte)) {
	lengths := []int{1, 8, 32}
	for i := 64; i < 1<<19; i <<= 2 {
		lengths = append(lengths, i)
	}
	for _, i := range lengths {
		b.Run(fmt.Sprintf("size=%d", i), func(b *testing.B) {
			data := make([]byte, i)
			for i := 0; i < b.N; i++ {
				hasher(data)
			}
		})
	}
}

func BenchmarkKeccakGo(b *testing.B) {
	benchmark(b, func(data []byte) {
		keccak256_Go(data)
	})
}

func BenchmarkKeccakC(b *testing.B) {
	benchmark(b, func(data []byte) {
		keccak256_C(data)
	})
}

func BenchmarkKeccakGo32ByteGeneric(b *testing.B) {
	data := [32]byte{}
	for i := 0; i < b.N; i++ {
		keccak256_Go(data[:])
	}
}

func BenchmarkKeccakC32ByteGeneric(b *testing.B) {
	data := [32]byte{}
	for i := 0; i < b.N; i++ {
		keccak256_C(data[:])
	}
}

func BenchmarkKeccakC32ByteSpecialized(b *testing.B) {
	data := [32]byte{}
	for i := 0; i < b.N; i++ {
		keccak256_C_32byte(data)
	}
}
