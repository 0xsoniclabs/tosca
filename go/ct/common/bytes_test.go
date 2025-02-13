// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package common

import (
	"bytes"
	"encoding/json"
	"testing"

	"pgregory.net/rand"
)

func TestBytes_EqualWhenContainingSameContent(t *testing.T) {
	b1 := NewBytes([]byte{1, 2, 3})
	b2 := NewBytes([]byte{1, 2, 3})
	b3 := NewBytes([]byte{3, 2, 1})

	if &b1 == &b2 {
		t.Fatalf("instances are not distinct, got %v and %v", &b1, &b2)
	}

	if b1 != b2 {
		t.Errorf("instances are not equal, got %v and %v", b1, b2)
	}

	if b1 == b3 {
		t.Errorf("instances are equal, got %v and %v", b1, b3)
	}
}

func TestBytes_AssignmentProducesEqualValue(t *testing.T) {
	b1 := NewBytes([]byte{1, 2, 3})
	b2 := b1

	if b1 != b2 {
		t.Errorf("assigned value is not equal: %v vs %v", b1, b2)
	}
}

func TestBytes_CanBeJsonEncoded(t *testing.T) {
	tests := map[string]struct {
		bytes   Bytes
		encoded string
	}{
		"empty": {
			NewBytes(nil), "\"\"",
		},
		"single": {
			NewBytes([]byte{1}), "\"01\"",
		},
		"triple": {
			NewBytes([]byte{1, 2, 3}), "\"010203\"",
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			encoded, err := json.Marshal(test.bytes)
			if err != nil {
				t.Fatalf("failed to encode into JSON: %v", err)
			}

			if want, got := test.encoded, string(encoded); want != got {
				t.Errorf("unexpected JSON encoding, wanted %v, got %v", want, got)
			}

			var restored Bytes
			if err := json.Unmarshal(encoded, &restored); err != nil {
				t.Fatalf("failed to restore bytes: %v", err)
			}
			if test.bytes != restored {
				t.Errorf("unexpected restored value, wanted %v, got %v", test.bytes, restored)
			}
		})
	}
}

func TestBytes_InvalidJsonFails(t *testing.T) {
	tests := map[string]string{
		"empty":               "",
		"missing start quote": "12\"",
		"missing end quote":   "\"12",
		"uneven length":       "\"123\"",
		"not hex":             "\"xy\"",
	}

	for name, encoded := range tests {
		t.Run(name, func(t *testing.T) {
			var restored Bytes
			err := json.Unmarshal([]byte(encoded), &restored)
			if err == nil {
				t.Errorf("decoding should have failed, but got %v", restored)
			}
		})
	}
}

func TestBytes_RandomBytesProducesRandom(t *testing.T) {
	rnd := rand.New()
	bytes := []Bytes{}
	for i := 0; i < 5; i++ {
		bytes = append(bytes, RandomBytes(rnd, 100))
		for j := 0; j < i; j++ {
			if bytes[i] == bytes[j] {
				t.Errorf("random bytes are not random, got %v and %v", bytes[i], bytes[j])
			}
		}
	}

}

func TestBytes_String(t *testing.T) {
	b := NewBytes([]byte{1, 2, 3})
	if b.String() != "0x010203" {
		t.Errorf("unexpected string representation, got %v", b.String())
	}
	if NewBytes([]byte{}).String() != "0x" {
		t.Errorf("unexpected string representation, got %v", Bytes{}.String())
	}
}

func TestBytes_Length(t *testing.T) {
	b := NewBytes([]byte{1, 2, 3})
	if b.Length() != 3 {
		t.Errorf("unexpected length, got %v", b.Length())
	}
	if NewBytes([]byte{}).Length() != 0 {
		t.Errorf("unexpected length, got %v", Bytes{}.Length())
	}
}

func TestBytes_Get(t *testing.T) {
	b := NewBytes([]byte{1, 2, 3})
	if got := b.Get(1, 2); !bytes.Equal(got, []byte{2}) {
		t.Errorf("unexpected slice, got %v", got)
	}
	if got := b.Get(0, 3); !bytes.Equal(got, []byte{1, 2, 3}) {
		t.Errorf("unexpected slice, got %v", got)
	}
	if got := b.Get(0, 0); !bytes.Equal(got, []byte{}) {
		t.Errorf("unexpected slice, got %v", got)
	}
}

func TestBytes_EqHandlesEmptyAndNilTheSameWay(t *testing.T) {
	tests := map[string]struct {
		first  Bytes
		second Bytes
		equal  bool
	}{
		"both empty": {
			first:  NewBytes([]byte{}),
			second: NewBytes([]byte{}),
			equal:  true,
		},
		"first empty": {
			first:  NewBytes([]byte{}),
			second: NewBytes([]byte{1}),
			equal:  false,
		},
		"second empty": {
			first:  NewBytes([]byte{1}),
			second: NewBytes([]byte{}),
			equal:  false,
		},
		"both nil": {
			first:  Bytes{},
			second: Bytes{},
			equal:  true,
		},
		"first nil": {
			first:  Bytes{},
			second: NewBytes([]byte{}),
			equal:  true,
		},
		"second nil": {
			first:  NewBytes([]byte{}),
			second: Bytes{},
			equal:  true,
		},
		"nil and non empty": {
			first:  Bytes{},
			second: NewBytes([]byte{1}),
			equal:  false,
		},
	}
	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			if got := test.first.Eq(test.second); got != test.equal {
				t.Errorf("unexpected equality, got %v", got)
			}
		})
	}
}
