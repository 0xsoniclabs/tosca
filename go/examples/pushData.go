package examples

import "github.com/0xsoniclabs/tosca/go/tosca/vm"

func GetPushDataExample() Example {
	iterations := 100
	code := make([]byte, 0, iterations*34+5)
	for range iterations {
		code = append(code, []byte{
			byte(vm.PUSH32),
			byte(0), byte(1), byte(2), byte(3), byte(4), byte(5), byte(6), byte(7), byte(8),
			byte(9), byte(10), byte(11), byte(12), byte(13), byte(14), byte(15), byte(16),
			byte(17), byte(18), byte(19), byte(20), byte(21), byte(22), byte(23), byte(24),
			byte(25), byte(26), byte(27), byte(28), byte(29), byte(30), byte(31),
			byte(vm.POP),
		}...)
	}
	code = append(code, []byte{
		byte(vm.PUSH1), 32,
		byte(vm.PUSH1), 0,
		byte(vm.RETURN),
	}...)

	return exampleSpec{
		Name:      "pushData",
		Code:      code,
		reference: pushData,
	}.build()
}

func pushData(_ int) int {
	return 0
}
