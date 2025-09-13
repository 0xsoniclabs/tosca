package sexpr

import (
	"fmt"
	"strings"
)

// Expression is a type used for representing conditions and
// expressions in S-expr notation. The corresponding syntax is used by the
// SMT-LIB format to represent formulas.
// For details see: https://en.wikipedia.org/wiki/S-expression
type Expression interface {
	fmt.Stringer
	_isExpression()
}

func Atom(value string) atom {
	return atom{value: value}
}

func List(first any, rest ...any) list {
	elements := make([]Expression, 0, len(rest)+1)
	elements = append(elements, toExpression(first))
	for _, cur := range rest {
		elements = append(elements, toExpression(cur))
	}
	return list{elements: elements}
}

func toExpression(value any) Expression {
	switch v := value.(type) {
	case Expression:
		return v
	case Expressioner:
		return v.Expression()
	case string:
		return Atom(v)
	default:
		return Atom(fmt.Sprintf("%v", value))
		//panic(fmt.Sprintf("unsupported type: %T", value))
	}
}

type atom struct {
	value string
}

func (a atom) String() string {
	return a.value
}

func (a atom) _isExpression() {}

type list struct {
	elements []Expression
}

func (l list) String() string {
	var sb strings.Builder
	sb.WriteString("(")
	entries := make([]string, len(l.elements))
	for i, element := range l.elements {
		entries[i] = element.String()
	}
	sb.WriteString(strings.Join(entries, " "))
	sb.WriteString(")")
	return sb.String()
}

func (l list) _isExpression() {}

type Expressioner interface {
	Expression() Expression
}
