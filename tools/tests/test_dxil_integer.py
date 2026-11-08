"""Check the independent DXIL control-flow probe interpreter's strict boundaries."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from dxil_integer import interpret_integer_stores


HANDLE = "%handle = call %dx.types.Handle @dx.op.createHandle(i32 57, i8 1, i32 0, i32 0, i1 false)"


def store(value, index=0):
    return f"call void @dx.op.bufferStore.i32(i32 69, %dx.types.Handle %handle, i32 {index}, i32 0, i32 {value}, i32 undef, i32 undef, i32 undef, i8 1)"


def program(body):
    return f"define void @main() {{\n{HANDLE}\n{body}\n}}\n"


class DxilIntegerTests(unittest.TestCase):
    def test_loop_phi_parameters_use_simultaneous_predecessor_values(self):
        body = """br label %loop
loop:
%a = phi i32 [ 1, %0 ], [ %b, %loop ]
%b = phi i32 [ 2, %0 ], [ %a, %loop ]
%i = phi i32 [ 0, %0 ], [ %next, %loop ]
%next = add nsw i32 %i, 1
%again = icmp slt i32 %next, 2
br i1 %again, label %loop, label %exit
exit:
""" + store("%a") + "\n" + store("%b", 1) + "\nret void"
        self.assertEqual(interpret_integer_stores(program(body)), {"0": 2, "1": 1})

    def test_undefined_values_are_only_allowed_in_unobserved_paths(self):
        body = "%value = select i1 true, i32 -1, i32 undef\n" + store("%value") + "\nret void"
        self.assertEqual(interpret_integer_stores(program(body)), {"0": 0xffffffff})
        for invalid in [body.replace("i1 true", "i1 false"), "br i1 undef, label %a, label %b", store("undef") + "\nret void"]:
            with self.assertRaises(ValueError):
                interpret_integer_stores(program(invalid))

    def test_signed_conversions_division_and_unsigned_comparisons(self):
        body = "%a = sext i16 -3 to i32\n%b = sdiv i32 %a, 2\n%c = srem i32 %a, 2\n%test = icmp ugt i32 %b, 4\n%d = zext i1 %test to i32\n"
        body += store("%b") + "\n" + store("%c", 1) + "\n" + store("%d", 2) + "\nret void"
        self.assertEqual(interpret_integer_stores(program(body)), {"0": 0xffffffff, "1": 0xffffffff, "2": 1})

    def test_poison_and_invalid_shifts_cannot_be_observed(self):
        for value in ["add nsw i32 2147483647, 1", "add nuw i32 -1, 1", "shl i32 1, 32", "sdiv i32 -2147483648, -1", "udiv i32 2, 0"]:
            with self.assertRaises(ValueError):
                interpret_integer_stores(program("%value = " + value + "\n" + store("%value") + "\nret void"))
        self.assertEqual(interpret_integer_stores(program("%value = add i32 -1, 1\n" + store("%value") + "\nret void")), {"0": 0})

    def test_unsupported_instructions_resources_and_missing_values_fail(self):
        for body in ["%a = call i32 @unknown()\nret void", "%a = load i32, i32* %ptr\nret void", store("%missing") + "\nret void", store("1").replace("i8 1)", "i8 3)") + "\nret void", "%other = call %dx.types.Handle @dx.op.createHandle(i32 57, i8 1, i32 1, i32 0, i1 false)\nret void"]:
            with self.assertRaises(ValueError):
                interpret_integer_stores(program(body))

    def test_nonterminating_loops_exhaust_an_explicit_budget(self):
        with self.assertRaisesRegex(ValueError, "budget"):
            interpret_integer_stores(program("br label %loop\nloop:\nbr label %loop"), max_steps=20)

    def test_switch_selects_exact_case_and_default_successors(self):
        body = """switch i32 -1, label %other [
  i32 -1, label %selected
  i32 0, label %other
]
selected:
""" + store("7") + "\nret void\nother:\n" + store("9") + "\nret void"
        self.assertEqual(interpret_integer_stores(program(body)), {"0": 7})
        self.assertEqual(interpret_integer_stores(program(body.replace("switch i32 -1", "switch i32 4"))), {"0": 9})


if __name__ == "__main__":
    unittest.main()
