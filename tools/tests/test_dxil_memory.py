"""Check raw-memory bounds, typed metadata, atomic values and undefined inputs."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from dxil_integer import interpret_integer_stores

METADATA = """!dx.resources = !{!0}
!0 = !{null, !1, null, null}
!1 = !{!2, !3}
!2 = !{i32 7, %struct.RWByteAddressBuffer* undef, !"", i32 0, i32 3, i32 1, i32 11, i1 false, i1 false, i1 false, null}
!3 = !{i32 9, %struct.Output* undef, !"", i32 0, i32 5, i32 1, i32 12, i1 false, i1 false, i1 false, !4}
!4 = !{i32 1, i32 4}
"""
HANDLES = """%raw = call %dx.types.Handle @dx.op.createHandle(i32 57, i8 1, i32 7, i32 3, i1 false)
%out = call %dx.types.Handle @dx.op.createHandle(i32 57, i8 1, i32 9, i32 5, i1 false)
"""


def raw_store(value, address=0, native=False):
    operation, opcode, tail = (
        ("rawBufferStore", 140, ", i32 4") if native else ("bufferStore", 69, "")
    )
    return f"call void @dx.op.{operation}.i32(i32 {opcode}, %dx.types.Handle %raw, i32 {address}, i32 undef, i32 {value}, i32 undef, i32 undef, i32 undef, i8 1{tail})"


def output(value, index=0):
    return f"call void @dx.op.bufferStore.i32(i32 69, %dx.types.Handle %out, i32 {index}, i32 0, i32 {value}, i32 undef, i32 undef, i32 undef, i8 1)"


def load(name="loaded", address=0, native=False):
    operation, opcode, tail = (
        ("rawBufferLoad", 139, ", i8 1, i32 4") if native else ("bufferLoad", 68, "")
    )
    return f"%{name} = call %dx.types.ResRet.i32 @dx.op.{operation}.i32(i32 {opcode}, %dx.types.Handle %raw, i32 {address}, i32 undef{tail})\n%word = extractvalue %dx.types.ResRet.i32 %{name}, 0"


def program(body):
    return f"define void @main() {{\n{HANDLES}{body}\nret void\n}}\n{METADATA}"


def interpret(body, **options):
    return interpret_integer_stores(program(body), raw_uav_bytes={"3": 16}, **options)


class DxilMemoryTests(unittest.TestCase):
    def test_resource_ids_and_registers_are_independently_validated(self):
        body = raw_store(23) + "\n" + load() + "\n" + output("%word")
        self.assertEqual(interpret(body), {"0": 23})
        with self.assertRaisesRegex(ValueError, "binding"):
            interpret_integer_stores(
                program(body).replace("i32 7, i32 3, i1 false", "i32 7, i32 4, i1 false"),
                raw_uav_bytes={"3": 16},
            )
        with self.assertRaises(ValueError):
            interpret_integer_stores(program(body))

    def test_all_atomic_word_operations_return_the_original_bits(self):
        # Negative words distinguish signed and unsigned min/max independently.
        for operation, expected in [
            (0, 1),
            (1, 2),
            (2, 0xFFFFFFFF),
            (3, 0xFFFFFFFD),
            (4, 0xFFFFFFFE),
            (5, 3),
            (6, 3),
            (7, 0xFFFFFFFE),
            (8, 3),
        ]:
            with self.subTest(operation=operation):
                body = raw_store(-2)
                body += f"\n%old = call i32 @dx.op.atomicBinOp.i32(i32 78, %dx.types.Handle %raw, i32 {operation}, i32 0, i32 undef, i32 undef, i32 3)"
                body += "\n" + load() + "\n" + output("%old") + "\n" + output("%word", 1)
                self.assertEqual(interpret(body), {"0": 0xFFFFFFFE, "1": expected})

    def test_compare_exchange_updates_only_when_the_original_matches(self):
        for comparison, expected in [(17, 29), (18, 17)]:
            body = raw_store(17)
            body += f"\n%old = call i32 @dx.op.atomicCompareExchange.i32(i32 79, %dx.types.Handle %raw, i32 0, i32 undef, i32 undef, i32 {comparison}, i32 29)"
            body += "\n" + load() + "\n" + output("%old") + "\n" + output("%word", 1)
            self.assertEqual(interpret(body), {"0": 17, "1": expected})

    def test_native_word_opcodes_preserve_raw_byte_addresses(self):
        body = raw_store(41, 12, native=True) + "\n" + load(address=12, native=True)
        body += "\n" + output("%word")
        self.assertEqual(interpret(body), {"0": 41})

    def test_atomic_memory_survives_loop_phi_transfers(self):
        body = raw_store(4) + "\nbr label %loop\nloop:\n"
        body += "%index = phi i32 [ 0, %0 ], [ %next, %loop ]\n"
        body += "%old = call i32 @dx.op.atomicBinOp.i32(i32 78, %dx.types.Handle %raw, i32 0, i32 0, i32 undef, i32 undef, i32 2)\n"
        body += "%next = add i32 %index, 1\n%again = icmp ult i32 %next, 3\n"
        body += "br i1 %again, label %loop, label %exit\nexit:\n"
        body += load() + "\n" + output("%old") + "\n" + output("%word", 1)
        self.assertEqual(interpret(body), {"0": 8, "1": 10})

    def test_uninitialized_unaligned_and_out_of_bounds_accesses_fail(self):
        for body in [load(), raw_store(1, 1), raw_store(1, 16), raw_store(1, -4)]:
            with self.subTest(body=body), self.assertRaises(ValueError):
                interpret(body)
        with self.assertRaisesRegex(ValueError, "initialization"):
            interpret(
                "%old = call i32 @dx.op.atomicBinOp.i32(i32 78, %dx.types.Handle %raw, i32 0, i32 0, i32 undef, i32 undef, i32 3)"
            )

    def test_unknown_layouts_contracts_vectors_and_undefined_values_fail(self):
        body = raw_store(1) + "\n" + load() + "\n" + output("%word")
        for sizes in [[], {}, {"4": 16}, {"3": 3}, {"3": True}, {"3": 0x100000004}]:
            with self.subTest(sizes=sizes), self.assertRaises(ValueError):
                interpret_integer_stores(program(body), raw_uav_bytes=sizes)
        for changed in [
            program(body).replace("i32 1, i32 4}", "i32 1, i32 8}"),
            program(body).replace("i32 0, i32 5, i32 1", "i32 0, i32 3, i32 1"),
            program(body).replace("%loaded, 0", "%loaded, 1"),
            program(body).replace("i32 1, i32 undef", "i32 undef, i32 undef"),
        ]:
            with self.assertRaises(ValueError):
                interpret_integer_stores(changed, raw_uav_bytes={"3": 16})


if __name__ == "__main__":
    unittest.main()
