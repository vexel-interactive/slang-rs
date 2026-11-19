"""Explicitly bounded, sequential raw-word memory for integer DXIL probes.

This models one invocation with initialized scalar words, not concurrent GPU
execution. Contracts name raw UAV registers in space zero and their byte sizes.
DXIL metadata must independently establish those bindings and one scalar-word
structured output. Unknown storage, reads before writes, and vector/narrow
accesses are rejected instead of being silently approximated.
"""

import re

NAME = r"%[-\w.$]+"
OPERAND = rf"(?:{NAME}|-?\d+|true|false|undef|poison)"


class RawUavMemory:
    def __init__(self, dump, byte_sizes):
        self.bindings = {}
        self.words = {}
        self.sizes = {}
        if not isinstance(byte_sizes, dict):
            raise ValueError("raw UAV byte-size contracts must be a register map")
        for register, size in byte_sizes.items():
            if not isinstance(register, str) or not register.isdecimal():
                raise ValueError("raw UAV contracts require decimal space-zero registers")
            if type(size) is not int or not 0 < size <= 0x100000000 or size % 4:
                raise ValueError("raw UAV byte sizes must be positive whole-word bounds")
            index = int(register)
            if index in self.sizes:
                raise ValueError("duplicate raw UAV register contract")
            self.sizes[index] = size
            self.words[index] = {}
        if not self.sizes:
            raise ValueError("raw UAV interpretation requires byte-size contracts")

        metadata = dict(re.findall(r"^!(\d+) = !\{(.*)\}$", dump, re.M))
        root = re.search(r"^!dx.resources = !\{!(\d+)\}$", dump, re.M)
        if root is None or root[1] not in metadata:
            raise ValueError("missing DXIL resource metadata")
        groups = metadata[root[1]].split(", ")
        if len(groups) != 4 or not re.fullmatch(r"!\d+", groups[1]):
            raise ValueError("missing DXIL UAV metadata group")
        entries = metadata.get(groups[1][1:], "")
        if not re.fullmatch(r"!\d+(?:, !\d+)*", entries):
            raise ValueError("unsupported DXIL UAV metadata group")
        raw_registers = set()
        used_registers = set()
        outputs = 0
        for reference in entries.split(", "):
            entry = metadata.get(reference[1:], "")
            match = re.fullmatch(
                r'i32 (\d+), [^,]+, !"[^"\n]*", i32 (\d+), i32 (\d+), '
                r"i32 (\d+), i32 (\d+), i1 false, i1 false, i1 false, (null|!\d+)",
                entry,
            )
            if match is None:
                raise ValueError("unsupported DXIL UAV resource metadata")
            identifier, space, register, count, kind, properties = match.groups()
            identifier, space, register, count, kind = map(
                int, [identifier, space, register, count, kind]
            )
            if space or count != 1 or identifier in self.bindings:
                raise ValueError("raw probes require distinct single UAVs in space zero")
            if register in used_registers:
                raise ValueError("overlapping DXIL UAV bindings")
            used_registers.add(register)
            if kind == 11 and register in self.sizes:
                raw_registers.add(register)
                storage = ("raw", register)
            elif kind == 12 and metadata.get(properties[1:]) == "i32 1, i32 4":
                outputs += 1
                storage = ("output", register)
            else:
                raise ValueError("missing raw bounds or unsupported structured output stride")
            self.bindings[identifier] = storage
        if raw_registers != self.sizes.keys() or outputs != 1:
            raise ValueError("raw contracts require exact bindings and one word output UAV")

    def location(self, handle, address, values):
        storage = values.get(handle)
        if not isinstance(storage, tuple) or storage[0] != "raw":
            raise ValueError("raw operation requires a contracted raw UAV")
        register = storage[1]
        if address % 4 or address + 4 > self.sizes[register]:
            raise ValueError("raw word access is unaligned or outside its byte bound")
        return self.words[register], address

    def read(self, handle, address, values):
        words, address = self.location(handle, address, values)
        if address not in words:
            raise ValueError("raw word read before initialization")
        return words[address]

    def step(self, instruction, values, observed, stores):
        if match := re.fullmatch(
            rf"({NAME}) = call %dx.types.Handle @dx.op.createHandle\(i32 57, i8 1, i32 (\d+), i32 (\d+), i1 false\)",
            instruction,
        ):
            name, identifier, register = match.groups()
            storage = self.bindings.get(int(identifier))
            if storage is None or storage[1] != int(register):
                raise ValueError("DXIL handle does not match its resource binding")
            values[name] = storage
            return True
        if match := re.fullmatch(
            rf"call void @dx.op.(bufferStore|rawBufferStore).i32\(i32 (69|140), %dx.types.Handle ({NAME}), i32 ({OPERAND}), i32 (undef|0), i32 ({OPERAND}), i32 undef, i32 undef, i32 undef, i8 1(, i32 4)?\)",
            instruction,
        ):
            operation, opcode, handle, index, component, value, alignment = match.groups()
            if (operation, opcode, alignment) not in {
                ("bufferStore", "69", None),
                ("rawBufferStore", "140", ", i32 4"),
            }:
                raise ValueError("inconsistent DXIL scalar store opcode")
            storage = values.get(handle)
            if not isinstance(storage, tuple) or storage[0] not in {"raw", "output"}:
                raise ValueError("unknown DXIL store resource")
            index, value = observed(index, 32), observed(value, 32)
            if storage[0] == "raw" and component == "undef":
                words, address = self.location(handle, index, values)
                words[address] = value
            elif storage[0] == "output" and component == "0":
                stores[str(index)] = value
            else:
                raise ValueError("DXIL store coordinates do not match resource storage")
            return True
        if match := re.fullmatch(
            rf"({NAME}) = call %dx.types.ResRet.i32 @dx.op.(bufferLoad|rawBufferLoad).i32\(i32 (68|139), %dx.types.Handle ({NAME}), i32 ({OPERAND}), i32 undef(, i8 1, i32 4)?\)",
            instruction,
        ):
            result, operation, opcode, handle, address, alignment = match.groups()
            if (operation, opcode, alignment) not in {
                ("bufferLoad", "68", None),
                ("rawBufferLoad", "139", ", i8 1, i32 4"),
            }:
                raise ValueError("inconsistent DXIL scalar load opcode")
            values[result] = ("raw-load", self.read(handle, observed(address, 32), values))
            return True
        if match := re.fullmatch(
            rf"({NAME}) = extractvalue %dx.types.ResRet.i32 ({NAME}), (\d+)", instruction
        ):
            result, source, component = match.groups()
            value = values.get(source)
            if not isinstance(value, tuple) or value[0] != "raw-load" or component != "0":
                raise ValueError("raw probes support only scalar integer load extraction")
            values[result] = value[1]
            return True
        if match := re.fullmatch(
            rf"({NAME}) = call i32 @dx.op.atomicBinOp.i32\(i32 78, %dx.types.Handle ({NAME}), i32 (\d+), i32 ({OPERAND}), i32 undef, i32 undef, i32 ({OPERAND})\)",
            instruction,
        ):
            result, handle, operation, address, value = match.groups()
            address, value = observed(address, 32), observed(value, 32)
            old = self.read(handle, address, values)
            signed = lambda number: number if number < 0x80000000 else number - 0x100000000
            operations = {
                "0": lambda: old + value,
                "1": lambda: old & value,
                "2": lambda: old | value,
                "3": lambda: old ^ value,
                "4": lambda: min(signed(old), signed(value)),
                "5": lambda: max(signed(old), signed(value)),
                "6": lambda: min(old, value),
                "7": lambda: max(old, value),
                "8": lambda: value,
            }
            if operation not in operations:
                raise ValueError("unsupported DXIL raw atomic operation")
            words, address = self.location(handle, address, values)
            words[address] = operations[operation]() & 0xFFFFFFFF
            values[result] = old
            return True
        if match := re.fullmatch(
            rf"({NAME}) = call i32 @dx.op.atomicCompareExchange.i32\(i32 79, %dx.types.Handle ({NAME}), i32 ({OPERAND}), i32 undef, i32 undef, i32 ({OPERAND}), i32 ({OPERAND})\)",
            instruction,
        ):
            result, handle, address, comparison, value = match.groups()
            address = observed(address, 32)
            old = self.read(handle, address, values)
            if old == observed(comparison, 32):
                words, address = self.location(handle, address, values)
                words[address] = observed(value, 32)
            else:
                observed(value, 32)
            values[result] = old
            return True
        return False
