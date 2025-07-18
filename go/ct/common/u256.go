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
	"fmt"
	"math/big"
	"regexp"
	"strconv"

	"pgregory.net/rand"

	"github.com/holiman/uint256"
)

// U256 is a 256-bit integer type. Contrary to holiman/uint256.Int the API
// operates on values rather than pointers.
type U256 struct {
	internal uint256.Int
}

// NewU256 creates a new U256 instance from up to 4 uint64 arguments. The
// arguments are given in the order from most significant to least significant
// by padding leading zeros as needed. No argument results in a value of zero.
func NewU256(args ...uint64) (result U256) {
	if len(args) > 4 {
		panic("Too many arguments")
	}
	offset := 4 - len(args)
	for i := 0; i < len(args) && i < len(result.internal); i++ {
		result.internal[3-i-offset] = args[i]
	}
	return
}

// NewU256FromBytes creates a new U256 instance from up to 32 byte arguments.
// The arguments are given in the order from most significant to least
// significant by padding leading zeros as needed. No argument results in a
// value of zero.
func NewU256FromBytes(bytes ...byte) (result U256) {
	if len(bytes) > 32 {
		panic("Too many arguments")
	}
	result.internal.SetBytes(bytes)
	return
}

// NewU256FromBigInt creates a new U256 instance from a big.Int.
// The constructor panics if the big.Int is negative or has more than 256 bits.
func NewU256FromBigInt(b *big.Int) (result U256) {
	if b.Cmp(big.NewInt(0)) == -1 {
		panic("Cannot construct U256 from negative big.Int")
	}
	overflow := result.internal.SetFromBig(b)
	if overflow {
		panic("Cannot construct U256 from big.Int with more than 256 bits")
	}
	return
}

// NewU256FromUint256 creates a new U256 instance from the given uint256.Int.
func NewU256FromUint256(value *uint256.Int) U256 {
	return U256{*value}
}

func RandU256(rnd *rand.Rand) U256 {
	var value U256
	value.internal[0] = rnd.Uint64()
	value.internal[1] = rnd.Uint64()
	value.internal[2] = rnd.Uint64()
	value.internal[3] = rnd.Uint64()
	return value
}

func RandU256Between(rnd *rand.Rand, min, max U256) U256 {
	if min.internal.Gt(&max.internal) {
		panic("min is greater than max")
	}
	// Calculate the range
	rangeValue := max.Sub(min).Add(NewU256(1))
	// Generate a random value within the range
	randValue := RandU256(rnd).Mod(rangeValue)
	// Add the min value to the random value
	return min.Add(randValue)
}

func MaxU256() (result U256) {
	result.internal.SetAllOne()
	return
}

func (a U256) IsZero() bool {
	return a.internal.IsZero()
}

func (a U256) IsUint64() bool {
	return a.internal.IsUint64()
}

func (a U256) Uint64() uint64 {
	return a.internal.Uint64()
}

func (a U256) Uint256() uint256.Int {
	return a.internal
}

func (a U256) Bytes32be() [32]byte {
	return a.internal.Bytes32()
}

func (a U256) Bytes20be() [20]byte {
	return a.internal.Bytes20()
}

func (a U256) Eq(b U256) bool {
	return a.internal.Eq(&b.internal)
}

func (a U256) Ne(b U256) bool {
	return !a.internal.Eq(&b.internal)
}

func (a U256) Lt(b U256) bool {
	return a.internal.Lt(&b.internal)
}

func (a U256) Slt(b U256) bool {
	return a.internal.Slt(&b.internal)
}

func (a U256) Gt(b U256) bool {
	return a.internal.Gt(&b.internal)
}

func (a U256) Sgt(b U256) bool {
	return a.internal.Sgt(&b.internal)
}

func (a U256) Add(b U256) (z U256) {
	z.internal.Add(&a.internal, &b.internal)
	return
}

func (a U256) AddMod(b, m U256) (z U256) {
	z.internal.AddMod(&a.internal, &b.internal, &m.internal)
	return
}

func (a U256) Sub(b U256) (z U256) {
	z.internal.Sub(&a.internal, &b.internal)
	return
}

func (a U256) Mul(b U256) (z U256) {
	z.internal.Mul(&a.internal, &b.internal)
	return
}

func (a U256) MulMod(b, m U256) (z U256) {
	z.internal.MulMod(&a.internal, &b.internal, &m.internal)
	return
}

func (a U256) Div(b U256) (z U256) {
	z.internal.Div(&a.internal, &b.internal)
	return
}

func (a U256) SDiv(b U256) (z U256) {
	z.internal.SDiv(&a.internal, &b.internal)
	return
}

func (a U256) Mod(b U256) (z U256) {
	z.internal.Mod(&a.internal, &b.internal)
	return
}

func (a U256) SMod(b U256) (z U256) {
	z.internal.SMod(&a.internal, &b.internal)
	return
}

func (a U256) Exp(b U256) (z U256) {
	z.internal.Exp(&a.internal, &b.internal)
	return
}

func (a U256) SignExtend(b U256) (z U256) {
	z.internal.ExtendSign(&a.internal, &b.internal)
	return
}

func (a U256) And(b U256) (z U256) {
	z.internal.And(&a.internal, &b.internal)
	return
}

func (a U256) Or(b U256) (z U256) {
	z.internal.Or(&a.internal, &b.internal)
	return
}

func (a U256) Xor(b U256) (z U256) {
	z.internal.Xor(&a.internal, &b.internal)
	return
}

func (a U256) Not() (z U256) {
	z.internal.Not(&a.internal)
	return
}

func (a U256) Shl(b U256) (z U256) {
	if b.internal.LtUint64(256) {
		z.internal.Lsh(&a.internal, uint(b.internal.Uint64()))
	}
	return
}

func (a U256) Shr(b U256) (z U256) {
	if b.internal.LtUint64(256) {
		z.internal.Rsh(&a.internal, uint(b.internal.Uint64()))
	}
	return
}

func (a U256) Srsh(b U256) (z U256) {
	if b.internal.GtUint64(256) {
		if a.internal.IsZero() || a.internal.Sign() >= 0 {
			return NewU256(0)
		}
		return MaxU256()
	}
	z.internal.SRsh(&a.internal, uint(b.internal.Uint64()))
	return
}

func (a U256) String() string {
	return fmt.Sprintf("%016x %016x %016x %016x", a.internal[3], a.internal[2], a.internal[1], a.internal[0])
}

func (a U256) DecimalString() string {
	return a.internal.String()
}

func (a U256) MarshalText() ([]byte, error) {
	return []byte(a.String()), nil
}

func (a *U256) UnmarshalText(text []byte) error {
	r := regexp.MustCompile("^([[:xdigit:]]{16}) ([[:xdigit:]]{16}) ([[:xdigit:]]{16}) ([[:xdigit:]]{16})$")
	match := r.FindSubmatch(text)

	if len(match) != 5 {
		return fmt.Errorf("invalid U256: %s", text)
	}

	for j := 0; j < 4; j++ {
		var err error
		a.internal[j], err = strconv.ParseUint(string(match[4-j]), 16, 64)
		if err != nil {
			return fmt.Errorf("failed to parse U256 (%v): %s", err, text)
		}
	}
	return nil
}

// ToBigInt returns a bigInt version of i
func (a U256) ToBigInt() *big.Int {
	return a.internal.ToBig()
}
